import { assert } from "node:console"
import {
	appendTransactionMessageInstruction,
	assertIsSendableTransaction,
	assertIsTransactionWithBlockhashLifetime,
	createSignerFromKeyPair,
	createSolanaRpc,
	createSolanaRpcSubscriptions,
	createTransactionMessage,
	getBase58Decoder,
	getProgramDerivedAddress,
	getSignatureFromTransaction,
	type Instruction,
	type KeyPairSigner,
	pipe,
	sendAndConfirmTransactionFactory,
	setTransactionMessageFeePayerSigner,
	setTransactionMessageLifetimeUsingBlockhash,
	signTransactionMessageWithSigners
} from "@solana/kit"
import {
	estimateComputeUnitLimitFactory,
	getSetComputeUnitLimitInstruction,
	getSetComputeUnitPriceInstruction
} from "@solana-program/compute-budget"
import {
	CLOSE_ACCOUNT_PROGRAM_ADDRESS,
	fetchMaybeMeme,
	fetchMeme,
	getCloseMemeInstruction,
	getCreateMemeInstruction
} from "js-client/close-account"
import { getKeyPair } from "./keypair"

const rpc = createSolanaRpc("https://api.devnet.solana.com")

const rpcSubscriptions = createSolanaRpcSubscriptions("wss://api.devnet.solana.com")

const sendAndConfirmTransaction = sendAndConfirmTransactionFactory({
	rpc,
	rpcSubscriptions
})

const buildMessage = async (signer: KeyPairSigner<string>, instructions: Instruction[]) => {
	const latestBlockhash = await rpc.getLatestBlockhash().send()

	return pipe(
		createTransactionMessage({ version: 0 }),
		msg => setTransactionMessageFeePayerSigner(signer, msg),
		msg => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash.value, msg),
		msg =>
			appendTransactionMessageInstruction(
				getSetComputeUnitPriceInstruction({ microLamports: 100_000 }),
				msg
			),
		msg =>
			instructions.reduce(
				// biome-ignore lint/suspicious/noExplicitAny: <it's ok>
				(message, ix) => appendTransactionMessageInstruction(ix, message) as any,
				msg
			),
		async msg =>
			appendTransactionMessageInstruction(
				getSetComputeUnitLimitInstruction({
					units: await estimateComputeUnitLimitFactory({ rpc })(msg)
				}),
				msg
			)
	)
}

const buildAndSendTransaction = async (
	signer: KeyPairSigner<string>,
	instructions: Instruction[]
) => {
	const message = await buildMessage(signer, instructions)
	const transaction = await signTransactionMessageWithSigners(message)

	assertIsTransactionWithBlockhashLifetime(transaction)
	assertIsSendableTransaction(transaction)

	await sendAndConfirmTransaction(transaction, { commitment: "confirmed" })

	return getSignatureFromTransaction(transaction)
}

const client = {
	rpc,
	buildAndSendTransaction
}

const main = async () => {
	const payer = await createSignerFromKeyPair(await getKeyPair())

	const [memeAddr, bump] = await getProgramDerivedAddress({
		programAddress: CLOSE_ACCOUNT_PROGRAM_ADDRESS,
		seeds: [Buffer.from("meme")]
	})

	const memeAcc = await fetchMaybeMeme(rpc, memeAddr)

	if (!memeAcc.exists) {
		const createMemeIx = getCreateMemeInstruction({
			meme: memeAddr,
			payer
		})

		const createMemeSignature = await client.buildAndSendTransaction(payer, [createMemeIx])

		console.log("createMemeSignature: ", createMemeSignature)
	}

	const meme = await fetchMeme(rpc, memeAddr)

	assert(getBase58Decoder().decode(meme.data.address) === memeAddr)

	assert(meme.data.bump, bump)
	assert(meme.data.address, memeAddr)

	const closeMemeIx = getCloseMemeInstruction({ meme: memeAddr, payer })

	const closeMemeSignature = await client.buildAndSendTransaction(payer, [closeMemeIx])

	console.log("closeMemeSignature: ", closeMemeSignature)

	const memeAccAfterClose = await fetchMaybeMeme(rpc, memeAddr)

	assert(!memeAccAfterClose.exists)
}

main()
