import {
	appendTransactionMessageInstruction,
	assertIsSendableTransaction,
	assertIsTransactionWithBlockhashLifetime,
	createSolanaRpc,
	createSolanaRpcSubscriptions,
	createTransactionMessage,
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

export const rpc = createSolanaRpc("https://api.devnet.solana.com")

export const rpcSubscriptions = createSolanaRpcSubscriptions("wss://api.devnet.solana.com")

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

export const buildAndSendTransaction = async (
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
