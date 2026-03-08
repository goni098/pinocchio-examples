#!/usr/bin/env node
// Reads every IDL in idl/ and generates a JS client under js-client/<name>
// using codama + @codama/renderers-js programmatically (no JSON config files).

const { readFileSync, readdirSync } = require("node:fs")
const { join, basename } = require("node:path")
const { rootNodeFromAnchor } = require("@codama/nodes-from-anchor")
const { createFromRoot } = require("codama")
const { renderVisitor } = require("@codama/renderers-js")

const ROOT = join(__dirname, "..")
const IDL_DIR = join(ROOT, "idl")
const CLIENT_DIR = join(ROOT, "js-client")

// Map IDL filename stem -> JS client output directory name
const OUTPUT_MAP = {
	counter: "counter",
	close_account: "close-account",
	basic_mint: "basic-mint",
	transfer_mint: "vault"
}

const idlFiles = readdirSync(IDL_DIR).filter(f => f.endsWith(".json"))

for (const file of idlFiles) {
	const stem = basename(file, ".json")
	const outName = OUTPUT_MAP[stem] ?? stem
	const idlPath = join(IDL_DIR, file)
	const outDir = join(CLIENT_DIR, outName)

	console.log(`  codama: idl/${file} -> js-client/${outName}`)

	const idl = JSON.parse(readFileSync(idlPath, "utf8"))
	const root = rootNodeFromAnchor(idl)
	const codama = createFromRoot(root)

	codama.accept(
		renderVisitor(outDir, {
			generatedFolder: ".",
			syncPackageJson: false,
			deleteFolderBeforeRendering: false
		})
	)
}

console.log("  done.")
