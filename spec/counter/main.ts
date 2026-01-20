import { assert } from "node:console"
import {
	createSignerFromKeyPair,
	getBase58Decoder,
	getBase58Encoder,
	getProgramDerivedAddress
} from "@solana/kit"
import {
	COUNTER_PROGRAM_ADDRESS,
	fetchCounterAuthority,
	fetchMaybeCounterAuthority,
	getIncreaseCounterAuthorityInstruction,
	getInitCounterAuhthorityInstruction
} from "js-client/counter"
import { buildAndSendTransaction, rpc } from "spec/client"
import { getKeyPair } from "spec/keypair"

const main = async () => {
	const payer = await createSignerFromKeyPair(await getKeyPair())

	const [counterAddr, bump] = await getProgramDerivedAddress({
		programAddress: COUNTER_PROGRAM_ADDRESS,
		seeds: [Buffer.from("counter_authority"), getBase58Encoder().encode(payer.address)]
	})

	const counterAcc = await fetchMaybeCounterAuthority(rpc, counterAddr)
	let currentCount = 19n

	if (!counterAcc.exists) {
		const initCounterAuhthorityInstruction = getInitCounterAuhthorityInstruction({
			counterAuthority: counterAddr,
			payer,
			count: currentCount
		})

		const signature = await buildAndSendTransaction(payer, [initCounterAuhthorityInstruction])

		console.log("init counter authority signature: ", signature)

		const counter = await fetchCounterAuthority(rpc, counterAddr)

		assert(counter.data.bump === bump)
		assert(getBase58Decoder().decode(counter.data.authority) === payer.address)
	} else {
		currentCount = counterAcc.data.count
	}

	const increaseSignature = await buildAndSendTransaction(payer, [
		getIncreaseCounterAuthorityInstruction({
			authority: payer,
			counterAuthority: counterAddr
		})
	])

	console.log("increase counter authority signature: ", increaseSignature)

	const counterAfterIncrease = await fetchCounterAuthority(rpc, counterAddr)

	console.log("owner: ", counterAfterIncrease.address)

	assert(counterAfterIncrease.data.count === currentCount + 1n)
}

main()
