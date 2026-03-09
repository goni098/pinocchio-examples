import { assert } from "node:console"
import {
	createSignerFromKeyPair,
	generateKeyPairSigner,
	getBase58Decoder,
	getBase58Encoder,
	getProgramDerivedAddress
} from "@solana/kit"
import { getCreateAccountInstruction } from "@solana-program/system"
import {
	findAssociatedTokenPda,
	getCreateAssociatedTokenInstruction,
	getInitializeMint2Instruction,
	getMintToInstruction,
	TOKEN_PROGRAM_ADDRESS
} from "@solana-program/token"
import {
	fetchVault,
	getCreateVaultInstruction,
	getSwapExactInInstruction,
	getSwapExactOutInstruction,
	VAULT_PROGRAM_ADDRESS
} from "js-client/vault"
import { buildAndSendTransaction, rpc } from "spec/client"
import { getKeyPair } from "spec/keypair"

// SPL Token mint account size in bytes.
const MINT_SIZE = 82n

// Encode a base58 address string into its raw 32-byte representation.
const enc = getBase58Encoder()
// Decode raw bytes back into a base58 address string.
const dec = getBase58Decoder()

const main = async () => {
	const payer = await createSignerFromKeyPair(await getKeyPair())
	console.log("payer:", payer.address)

	// ── 1. Create two SPL Token mints ──────────────────────────────────────────
	const mintAKp = await generateKeyPairSigner()
	const mintBKp = await generateKeyPairSigner()

	const mintRentLamports = await rpc.getMinimumBalanceForRentExemption(MINT_SIZE).send()

	const createMintsSig = await buildAndSendTransaction(payer, [
		getCreateAccountInstruction({
			payer,
			newAccount: mintAKp,
			lamports: mintRentLamports,
			space: MINT_SIZE,
			programAddress: TOKEN_PROGRAM_ADDRESS
		}),
		getInitializeMint2Instruction({
			mint: mintAKp.address,
			decimals: 6,
			mintAuthority: payer.address
		}),
		getCreateAccountInstruction({
			payer,
			newAccount: mintBKp,
			lamports: mintRentLamports,
			space: MINT_SIZE,
			programAddress: TOKEN_PROGRAM_ADDRESS
		}),
		getInitializeMint2Instruction({
			mint: mintBKp.address,
			decimals: 6,
			mintAuthority: payer.address
		})
	])
	console.log("createMints sig:", createMintsSig)
	console.log("  mint A:", mintAKp.address)
	console.log("  mint B:", mintBKp.address)

	// ── 2. Create owner ATAs and mint initial supply ───────────────────────────
	const [userAtaA] = await findAssociatedTokenPda({
		owner: payer.address,
		mint: mintAKp.address,
		tokenProgram: TOKEN_PROGRAM_ADDRESS
	})
	const [userAtaB] = await findAssociatedTokenPda({
		owner: payer.address,
		mint: mintBKp.address,
		tokenProgram: TOKEN_PROGRAM_ADDRESS
	})

	const MINT_SUPPLY = 2_000_000n

	const setupSupplySig = await buildAndSendTransaction(payer, [
		getCreateAssociatedTokenInstruction({
			payer,
			ata: userAtaA,
			owner: payer.address,
			mint: mintAKp.address
		}),
		getCreateAssociatedTokenInstruction({
			payer,
			ata: userAtaB,
			owner: payer.address,
			mint: mintBKp.address
		}),
		getMintToInstruction({
			mint: mintAKp.address,
			token: userAtaA,
			mintAuthority: payer,
			amount: MINT_SUPPLY
		}),
		getMintToInstruction({
			mint: mintBKp.address,
			token: userAtaB,
			mintAuthority: payer,
			amount: MINT_SUPPLY
		})
	])
	console.log("setupSupply sig:", setupSupplySig)

	// ── 3. Derive vault PDA and create vault token ATAs ────────────────────────
	// Vault PDA seeds: ["vault", owner_pubkey_bytes, vault_id_bytes]
	const vaultIdKp = await generateKeyPairSigner()

	const [vaultPda] = await getProgramDerivedAddress({
		programAddress: VAULT_PROGRAM_ADDRESS,
		seeds: [Buffer.from("vault"), enc.encode(payer.address), enc.encode(vaultIdKp.address)]
	})
	console.log("vault PDA:", vaultPda)

	const [vaultAtaA] = await findAssociatedTokenPda({
		owner: vaultPda,
		mint: mintAKp.address,
		tokenProgram: TOKEN_PROGRAM_ADDRESS
	})
	const [vaultAtaB] = await findAssociatedTokenPda({
		owner: vaultPda,
		mint: mintBKp.address,
		tokenProgram: TOKEN_PROGRAM_ADDRESS
	})

	const createVaultAtasSig = await buildAndSendTransaction(payer, [
		getCreateAssociatedTokenInstruction({
			payer,
			ata: vaultAtaA,
			owner: vaultPda,
			mint: mintAKp.address
		}),
		getCreateAssociatedTokenInstruction({
			payer,
			ata: vaultAtaB,
			owner: vaultPda,
			mint: mintBKp.address
		})
	])
	console.log("createVaultAtas sig:", createVaultAtasSig)

	// ── 4. CreateVault: deposit 1_000_000 token A + 500_000 token B ───────────
	// k = 1_000_000 × 500_000 = 500_000_000_000
	const RESERVE_A = 1_000_000n
	const RESERVE_B = 500_000n

	const createVaultSig = await buildAndSendTransaction(payer, [
		getCreateVaultInstruction({
			owner: payer,
			vault: vaultPda,
			userTokenAccountA: userAtaA,
			userTokenAccountB: userAtaB,
			vaultTokenAccountA: vaultAtaA,
			vaultTokenAccountB: vaultAtaB,
			tokenProgramA: TOKEN_PROGRAM_ADDRESS,
			tokenProgramB: TOKEN_PROGRAM_ADDRESS,
			vaultId: enc.encode(vaultIdKp.address),
			tokenMintA: enc.encode(mintAKp.address),
			tokenMintB: enc.encode(mintBKp.address),
			amountA: RESERVE_A,
			amountB: RESERVE_B
		})
	])
	console.log("createVault sig:", createVaultSig)

	const vault = await fetchVault(rpc, vaultPda)
	assert(
		vault.data.reserveA === RESERVE_A,
		`reserve_a: expected ${RESERVE_A}, got ${vault.data.reserveA}`
	)
	assert(
		vault.data.reserveB === RESERVE_B,
		`reserve_b: expected ${RESERVE_B}, got ${vault.data.reserveB}`
	)
	assert(
		vault.data.k === RESERVE_A * RESERVE_B,
		`k: expected ${RESERVE_A * RESERVE_B}, got ${vault.data.k}`
	)
	assert(dec.decode(vault.data.tokenProgramA) === TOKEN_PROGRAM_ADDRESS, "token_program_a mismatch")
	assert(dec.decode(vault.data.tokenProgramB) === TOKEN_PROGRAM_ADDRESS, "token_program_b mismatch")
	console.log(
		"✓ CreateVault  reserve_a=%d  reserve_b=%d  k=%d",
		vault.data.reserveA,
		vault.data.reserveB,
		vault.data.k
	)

	// ── 5. SwapExactIn: send 100_000 token A, receive token B ─────────────────
	// k = 500_000_000_000
	// new_reserve_a = 1_000_000 + 100_000 = 1_100_000
	// new_reserve_b = floor(k / new_reserve_a) = floor(500_000_000_000 / 1_100_000) = 454_545
	// amount_out    = 500_000 - 454_545 = 45_455
	const SWAP_IN_AMOUNT = 100_000n
	const SWAP_IN_MIN_OUT = 40_000n // slippage guard

	const swapExactInSig = await buildAndSendTransaction(payer, [
		getSwapExactInInstruction({
			user: payer,
			vault: vaultPda,
			userTokenIn: userAtaA,
			vaultTokenIn: vaultAtaA,
			userTokenOut: userAtaB,
			vaultTokenOut: vaultAtaB,
			tokenProgramIn: TOKEN_PROGRAM_ADDRESS,
			tokenProgramOut: TOKEN_PROGRAM_ADDRESS,
			amountIn: SWAP_IN_AMOUNT,
			minAmountOut: SWAP_IN_MIN_OUT
		})
	])
	console.log("swapExactIn sig:", swapExactInSig)

	const vaultAfterIn = await fetchVault(rpc, vaultPda)
	assert(
		vaultAfterIn.data.reserveA === 1_100_000n,
		`reserve_a: expected 1_100_000, got ${vaultAfterIn.data.reserveA}`
	)
	assert(
		vaultAfterIn.data.reserveB === 454_545n,
		`reserve_b: expected 454_545, got ${vaultAfterIn.data.reserveB}`
	)
	console.log(
		"✓ SwapExactIn  reserve_a=%d  reserve_b=%d",
		vaultAfterIn.data.reserveA,
		vaultAfterIn.data.reserveB
	)

	// ── 6. SwapExactOut: receive exactly 20_000 token B, pay token A ──────────
	// reserves after SwapExactIn: reserve_a = 1_100_000, reserve_b = 454_545
	// k = 500_000_000_000 (never mutated)
	//
	// want_out    = 20_000 token B  (vault_token_out = vaultAtaB → A→B direction)
	// new_reserve_b = 454_545 - 20_000 = 434_545
	// new_reserve_a = floor(k / new_reserve_b) + 1       ← ceiling division
	//              = floor(500_000_000_000 / 434_545) + 1
	//              = 1_150_628 + 1 = 1_150_629
	// amount_in   = 1_150_629 - 1_100_000 = 50_629
	const SWAP_OUT_AMOUNT = 20_000n
	const SWAP_OUT_MAX_IN = 60_000n // slippage guard

	const swapExactOutSig = await buildAndSendTransaction(payer, [
		getSwapExactOutInstruction({
			user: payer,
			vault: vaultPda,
			userTokenIn: userAtaA,
			vaultTokenIn: vaultAtaA,
			userTokenOut: userAtaB,
			vaultTokenOut: vaultAtaB,
			tokenProgramIn: TOKEN_PROGRAM_ADDRESS,
			tokenProgramOut: TOKEN_PROGRAM_ADDRESS,
			amountOut: SWAP_OUT_AMOUNT,
			maxAmountIn: SWAP_OUT_MAX_IN
		})
	])
	console.log("swapExactOut sig:", swapExactOutSig)

	const vaultAfterOut = await fetchVault(rpc, vaultPda)
	assert(
		vaultAfterOut.data.reserveA === 1_150_629n,
		`reserve_a: expected 1_150_629, got ${vaultAfterOut.data.reserveA}`
	)
	assert(
		vaultAfterOut.data.reserveB === 434_545n,
		`reserve_b: expected 434_545, got ${vaultAfterOut.data.reserveB}`
	)
	console.log(
		"✓ SwapExactOut  reserve_a=%d  reserve_b=%d",
		vaultAfterOut.data.reserveA,
		vaultAfterOut.data.reserveB
	)
}

main()
