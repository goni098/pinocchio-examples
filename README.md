# Pinocchio Examples

A collection of example Solana programs built with [Pinocchio](https://github.com/febo/pinocchio), demonstrating efficient on-chain program development with minimal dependencies and optimized performance.

## 📚 Overview

This repository showcases practical examples of Solana programs written using the Pinocchio framework, which prioritizes performance and minimal runtime overhead by avoiding unnecessary abstractions. Each example includes both the on-chain program (Rust) and off-chain client code (TypeScript).

## 🚀 Examples

### 1. Counter Program

A demonstration of state management and Program Derived Addresses (PDAs) with two counter variants:

- **Simple Counter**: Basic counter that can be incremented by anyone
- **Authority Counter**: Counter with authority control, demonstrating access control patterns

**Features:**
- PDA-based account creation
- State initialization with custom data
- Permissioned and permissionless operations
- Event emission for state changes

**Location:** [`basic/counter`](basic/counter)

### 2. Close Account Program

Demonstrates the account lifecycle on Solana, including creation and proper account closure with rent refunds.

**Features:**
- PDA account creation
- Safe account closure
- Rent reclamation
- State validation

**Location:** [`basic/close-account`](basic/close-account)

## 🏗️ Project Structure

```
pinocchio-examples/
├── basic/                      # Basic example programs
│   ├── counter/               # Counter program with authority
│   └── close-account/         # Account lifecycle example
├── token/                     # Token-related examples
│   ├── create-mint/
│   └── transfer-mint/
├── js-client/                 # Generated TypeScript clients
│   ├── counter/              # Counter program client
│   └── close-account/        # Close account program client
├── spec/                      # Test specifications
│   ├── counter/
│   └── close-acount/
├── idl/                       # Program IDLs
├── deploy/                    # Deployment configurations
└── shared/                    # Shared Rust utilities
```

## 🛠️ Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Solana CLI**: Install from [Solana docs](https://docs.solana.com/cli/install-solana-cli-tools)
- **Node.js**: v18+ recommended
- **pnpm**: Package manager (v10.28.0+)

## 📦 Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd pinocchio-examples
```

2. Install dependencies:
```bash
pnpm install
```

3. Build the programs:
```bash
cargo build-sbf
```

## 🎯 Usage

### Building Programs

Build all programs in the workspace:
```bash
cargo build-sbf
```

Build release optimized programs:
```bash
cargo build-sbf --release
```

### Generating Client Code

Generate TypeScript client from program IDL:
```bash
pnpm run program:gen
```

Generate IDL from program:
```bash
pnpm run gen:idl
```

### Running Examples

Run the counter example:
```bash
npx tsx spec/counter/main.ts
```

Run the close account example:
```bash
npx tsx spec/close-acount/main.ts
```

### Deploying Programs

Deploy to devnet:
```bash
pnpm run deploy:program
```

Or deploy manually with Solana CLI:
```bash
solana program deploy \
  --program-id deploy/basic/counter/program.json \
  target/deploy/counter.so \
  --url devnet
```

## 🔧 Development

### Type Checking

Run TypeScript type checking:
```bash
pnpm run type-check
```

### Code Formatting

Format code with Biome:
```bash
pnpm run clean
```

## 📖 Key Concepts Demonstrated

### Pinocchio Framework
- **No-std environment**: Minimal runtime overhead
- **Direct syscall access**: Optimal performance
- **Type-safe account handling**: Using `AccountView`
- **Efficient serialization**: Using Borsh

### Solana Patterns
- **Program Derived Addresses (PDAs)**: Deterministic account addresses
- **Account initialization**: Creating and funding accounts
- **Account closure**: Properly closing accounts and reclaiming rent
- **Instruction routing**: Processing different instruction types
- **Event emission**: Logging program events

### Client Integration
- **@solana/kit**: Modern Solana TypeScript library
- **Codama**: IDL-based client generation
- **Transaction building**: Composing and sending transactions
- **Account fetching**: Reading on-chain state

## 🔑 Program IDs

- **Counter Program**: `8F1XtWR4wTs37nnutBvd2MWpCTfb7XAciFYkw5XHaENj`
- **Close Account Program**: `2HXWQuEjgRDbNcMx3X32C1aw4fftVMHyUf9KXYyTiPiD`

## 📚 Resources

- [Pinocchio Documentation](https://github.com/febo/pinocchio)
- [Solana Documentation](https://docs.solana.com/)
- [Codama Documentation](https://github.com/codama-idl/codama)
- [@solana/kit Documentation](https://github.com/anza-xyz/kit)

## 🤝 Contributing

Contributions are welcome! Feel free to submit issues or pull requests.

## 📄 License

ISC
