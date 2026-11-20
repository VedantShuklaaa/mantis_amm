# man-amm

A constant-product (x * y = k) automated market maker on Solana, built with Anchor.

Each pool is bound to one ordered pair of SPL mints (`mint_a < mint_b`). Liquidity is held in
two vault token accounts owned by the pool authority PDA, and pool shares are represented by a
dedicated LP mint.

## Instructions

| Instruction        | Description                                                        |
| ------------------ | ------------------------------------------------------------------ |
| `initialize_pool`  | Creates the pool, both vaults, the LP mint and the locked LP account |
| `add_liquidity`    | Deposits both tokens and mints LP shares                            |
| `remove_liquidity` | Burns LP shares and returns a pro-rata share of both reserves       |
| `swap`             | Trades one token for the other, taking a fee from the input        |

### Minimum liquidity

The first deposit mints `sqrt(a * b) - 1,000` LP to the user and 1,000 LP to `locked_lp` (a PDA
token account owned by `pool_authority`, which no instruction can move), so LP supply never
drops below 1,000, which blocks first-depositor price manipulation.

### Math and rounding

All arithmetic lives in `src/math.rs` and uses checked operations. Rounding always favors the
pool:

- Initial LP: `floor(sqrt(a * b)) - MINIMUM_LIQUIDITY`
- Subsequent deposits: `min(a * supply / reserve_a, b * supply / reserve_b)`, rounded down, with
  the amounts pulled from the user rounded up to match the pool ratio
- Withdrawals: `lp * reserve / supply`, rounded down
- Swaps: output rounded down with the fee taken from the input

`amount_in` for a swap is the full amount transferred into the input vault; the fee stays in the
vault, which increases `k`.

## State

`Pool` (`src/state.rs`):

| Field            | Type     | Description                              |
| ---------------- | -------- | ---------------------------------------- |
| `mint_a`         | `Pubkey` | Lower-sorted mint                        |
| `mint_b`         | `Pubkey` | Higher-sorted mint                       |
| `vault_a`        | `Pubkey` | Vault holding `mint_a`                   |
| `vault_b`        | `Pubkey` | Vault holding `mint_b`                   |
| `lp_mint`        | `Pubkey` | LP share mint                            |
| `fee_bps`        | `u16`    | Swap fee in basis points (max 100 = 1%)  |
| `pool_bump`      | `u8`     | Bump for the pool PDA                    |
| `authority_bump` | `u8`     | Bump for the pool authority PDA          |

### PDAs and seeds

| Account          | Seeds                                                    |
| ---------------- | -------------------------------------------------------- |
| `pool`           | `["pool", mint_a, mint_b]`                               |
| `pool_authority` | `["authority", pool]`                                    |
| `vault_a`        | `["vault", pool, mint_a]`                                |
| `vault_b`        | `["vault", pool, mint_b]`                                |
| `lp_mint`        | `["lp_mint", pool]`                                      |
| `locked_lp`      | `["locked_lp", pool]`                                    |

`locked_lp` is a token account for `lp_mint` whose authority is `pool_authority`. It receives
the permanently locked minimum liquidity on the first deposit.

## Build and test

Prerequisites: Rust, the Solana CLI, the Anchor CLI, Node.js and Yarn.

```bash
yarn install
anchor build
anchor test --provider.cluster localnet
```

The test suite lives in `tests/man-amm.ts` and is run through the script in `Anchor.toml`:

```toml
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
```

`anchor test` starts a local validator, deploys the program and runs the suite against it. The
tests need a localnet RPC; they create their own mints and token accounts and fund them from
the provider wallet.

Test dependencies (declared in `package.json`): `ts-mocha`, `mocha`, `chai`, `@solana/spl-token`
and the Anchor/Solana client packages. The TypeScript config targets ES2020 and includes
`tests/**/*.ts`.
