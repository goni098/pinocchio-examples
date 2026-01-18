import { assert } from "node:console"
import { createSignerFromKeyPair, getBase58Decoder, getProgramDerivedAddress } from "@solana/kit"
import {
	CLOSE_ACCOUNT_PROGRAM_ADDRESS,
	fetchMaybeMeme,
	fetchMeme,
	getCloseMemeInstruction,
	getCreateMemeInstruction
} from "js-client/close-account"
import { buildAndSendTransaction, rpc } from "../client"
import { getKeyPair } from "../keypair"

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

		const createMemeSignature = await buildAndSendTransaction(payer, [createMemeIx])

		console.log("createMemeSignature: ", createMemeSignature)
	}

	const meme = await fetchMeme(rpc, memeAddr)

	assert(getBase58Decoder().decode(meme.data.address) === memeAddr)

	assert(meme.data.bump, bump)
	assert(meme.data.address, memeAddr)

	const closeMemeIx = getCloseMemeInstruction({ meme: memeAddr, payer })

	const closeMemeSignature = await buildAndSendTransaction(payer, [closeMemeIx])

	console.log("closeMemeSignature: ", closeMemeSignature)

	const memeAccAfterClose = await fetchMaybeMeme(rpc, memeAddr)

	assert(!memeAccAfterClose.exists)
}

main()
