import { readFile } from "node:fs/promises"
import { createKeyPairFromBytes } from "@solana/kit"

export const getKeyPair = async () => {
	const file = await readFile("/Users/goni/.config/solana/id.json")
	const keyPairBytes = Uint8Array.from(JSON.parse(file.toString()))

	return createKeyPairFromBytes(keyPairBytes)
}
