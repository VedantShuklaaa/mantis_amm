import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import {
	createMint,
	getAccount,
	getMint,
	getOrCreateAssociatedTokenAccount,
	mintTo,
	TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";
import { ManAmm } from "../target/types/man_amm";
import { createAccount } from "@solana/spl-token";
import { Keypair } from "@solana/web3.js";


describe("man-amm", () => {
	const provider = anchor.AnchorProvider.env();
	anchor.setProvider(provider);
	const program = anchor.workspace.ManAmm as Program<ManAmm>;
	const conn = provider.connection;
	const payer = (provider.wallet as anchor.Wallet).payer;

	const FEE_BPS = 30;
	const BPS = 10_000n;
	const MIN_LIQ = 1_000n;

	// ---------- helpers ----------
	const bn = (x: bigint) => new BN(x.toString());
	const pda = (seeds: Buffer[]) =>
		PublicKey.findProgramAddressSync(seeds, program.programId);
	const bal = async (pk: PublicKey) => (await getAccount(conn, pk)).amount;

	async function expectErr(p: Promise<unknown>, code?: string) {
		try {
			await p;
		} catch (e: any) {
			if (code) {
				const got = e?.error?.errorCode?.code ?? String(e);
				assert.include(got, code);
			}
			return;
		}
		assert.fail(`expected failure${code ? ` (${code})` : ""}, but tx succeeded`);
	}

	async function sortedMints(): Promise<[PublicKey, PublicKey]> {
		const m1 = await createMint(conn, payer, payer.publicKey, null, 6);
		const m2 = await createMint(conn, payer, payer.publicKey, null, 9);
		return Buffer.compare(m1.toBuffer(), m2.toBuffer()) < 0 ? [m1, m2] : [m2, m1];
	}

	function derive(mintA: PublicKey, mintB: PublicKey) {
		const [pool, poolBump] = pda([Buffer.from("pool"), mintA.toBuffer(), mintB.toBuffer()]);
		const [poolAuthority, authorityBump] = pda([Buffer.from("authority"), pool.toBuffer()]);
		const [vaultA] = pda([Buffer.from("vault"), pool.toBuffer(), mintA.toBuffer()]);
		const [vaultB] = pda([Buffer.from("vault"), pool.toBuffer(), mintB.toBuffer()]);
		const [lpMint] = pda([Buffer.from("lp_mint"), pool.toBuffer()]);
		const [lockedLp] = pda([Buffer.from("locked_lp"), pool.toBuffer()]);
		return { pool, poolBump, poolAuthority, authorityBump, vaultA, vaultB, lpMint, lockedLp };
	}

	function initAccounts(mintA: PublicKey, mintB: PublicKey) {
		const d = derive(mintA, mintB);
		return {
			payer: payer.publicKey,
			mintA,
			mintB,
			pool: d.pool,
			poolAuthority: d.poolAuthority,
			vaultA: d.vaultA,
			vaultB: d.vaultB,
			lpMint: d.lpMint,
			lockedLp: d.lockedLp,
			systemProgram: SystemProgram.programId,
			tokenProgram: TOKEN_PROGRAM_ID,
		};
	}

	// off-chain mirrors of math.rs
	const swapOut = (amtIn: bigint, rIn: bigint, rOut: bigint, fee: number) => {
		const f = amtIn * (BPS - BigInt(fee));
		return (f * rOut) / (rIn * BPS + f);
	};

	// ---------- shared pool state for the ordered tests ----------
	let mintA: PublicKey, mintB: PublicKey;
	let d: ReturnType<typeof derive>;
	let userA: PublicKey, userB: PublicKey, userLp: PublicKey;

	const liqAccounts = () => ({
		user: payer.publicKey,
		pool: d.pool,
		poolAuthority: d.poolAuthority,
		vaultA: d.vaultA,
		vaultB: d.vaultB,
		lpMint: d.lpMint,
		lockedLp: d.lockedLp,
		userTokenA: userA,
		userTokenB: userB,
		userLp,
		tokenProgram: TOKEN_PROGRAM_ID,
	});

	const swapAccounts = () => ({
		user: payer.publicKey,
		pool: d.pool,
		poolAuthority: d.poolAuthority,
		vaultA: d.vaultA,
		vaultB: d.vaultB,
		userTokenA: userA,
		userTokenB: userB,
		tokenProgram: TOKEN_PROGRAM_ID,
	});

	const snap = async () => ({
		rA: await bal(d.vaultA),
		rB: await bal(d.vaultB),
		supply: (await getMint(conn, d.lpMint)).supply,
	});

	before(async () => {
		[mintA, mintB] = await sortedMints();
		d = derive(mintA, mintB);

		userA = (await getOrCreateAssociatedTokenAccount(conn, payer, mintA, payer.publicKey)).address;
		userB = (await getOrCreateAssociatedTokenAccount(conn, payer, mintB, payer.publicKey)).address;

		await mintTo(conn, payer, mintA, userA, payer, 1_000_000_000_000n);
		await mintTo(conn, payer, mintB, userB, payer, 1_000_000_000_000n);
	});

	// =====================================================
	describe("initialize_pool", () => {
		it("creates pool, vaults and LP mint", async () => {
			await program.methods
				.initializePool(FEE_BPS)
				.accounts(initAccounts(mintA, mintB) as any)
				.rpc();

			const pool = await program.account.pool.fetch(d.pool);
			assert.equal(pool.mintA.toBase58(), mintA.toBase58());
			assert.equal(pool.mintB.toBase58(), mintB.toBase58());
			assert.equal(pool.vaultA.toBase58(), d.vaultA.toBase58());
			assert.equal(pool.vaultB.toBase58(), d.vaultB.toBase58());
			assert.equal(pool.lpMint.toBase58(), d.lpMint.toBase58());
			assert.equal(pool.feeBps, FEE_BPS);
			assert.equal(pool.poolBump, d.poolBump);
			assert.equal(pool.authorityBump, d.authorityBump);

			const vA = await getAccount(conn, d.vaultA);
			const vB = await getAccount(conn, d.vaultB);
			assert.equal(vA.owner.toBase58(), d.poolAuthority.toBase58());
			assert.equal(vB.owner.toBase58(), d.poolAuthority.toBase58());
			assert.equal(vA.amount, 0n);
			assert.equal(vB.amount, 0n);

			const lp = await getMint(conn, d.lpMint);
			assert.equal(lp.decimals, 6);
			assert.equal(lp.mintAuthority?.toBase58(), d.poolAuthority.toBase58());
			assert.equal(lp.supply, 0n);
			assert.isNull(lp.freezeAuthority);
		});

		it("rejects unsorted mints", async () => {
			const [a, b] = await sortedMints();
			await expectErr(
				program.methods.initializePool(FEE_BPS).accounts(initAccounts(b, a) as any).rpc(),
				"MintsNotSorted"
			);
		});

		it("rejects identical mints", async () => {
			const [a] = await sortedMints();
			await expectErr(
				program.methods.initializePool(FEE_BPS).accounts(initAccounts(a, a) as any).rpc()
			);
		});

		it("rejects fee above the cap", async () => {
			const [a, b] = await sortedMints();
			await expectErr(
				program.methods.initializePool(101).accounts(initAccounts(a, b) as any).rpc(),
				"FeeTooHigh"
			);
		});

		it("rejects a duplicate pool for the same pair", async () => {
			await expectErr(
				program.methods.initializePool(50).accounts(initAccounts(mintA, mintB) as any).rpc()
			);
		});
	});

	// =====================================================
	describe("add_liquidity", () => {
		before(async () => {
			// program does not create the LP ATA, the client does
			userLp = (await getOrCreateAssociatedTokenAccount(conn, payer, d.lpMint, payer.publicKey)).address;
		});

		it("rejects zero amounts", async () => {
			await expectErr(
				program.methods.addLiquidity(bn(0n), bn(0n), bn(0n)).accounts(liqAccounts() as any).rpc(),
				"ZeroAmount"
			);
		});

		it("first deposit sets price and locks 1,000 LP units", async () => {
			const a = 100_000_000_000n; // 1e11 base units
			const b = 100_000_000_000n; // 1e11 base units
			const expectedLp = 100_000_000_000n - MIN_LIQ; // sqrt(1e22) - 1000

			await program.methods
				.addLiquidity(bn(a), bn(b), bn(expectedLp))
				.accounts(liqAccounts() as any)
				.rpc();

			const s = await snap();
			assert.equal(s.rA, a);
			assert.equal(s.rB, b);
			assert.equal(s.supply, expectedLp + MIN_LIQ);
			assert.equal(await bal(userLp), expectedLp);
		});

		it("second deposit is ratio-matched", async () => {
			const before = await snap();
			const userBefore = { a: await bal(userA), b: await bal(userB), lp: await bal(userLp) };

			// max_b is larger than needed, so only 10e9 of B should be pulled
			const maxA = 10_000_000_000n;
			const maxB = 20_000_000_000n;
			const lp = (maxA * before.supply) / before.rA;

			await program.methods
				.addLiquidity(bn(maxA), bn(maxB), bn(lp))
				.accounts(liqAccounts() as any)
				.rpc();

			assert.equal(userBefore.a - (await bal(userA)), maxA);
			assert.equal(userBefore.b - (await bal(userB)), 10_000_000_000n);
			assert.equal((await bal(userLp)) - userBefore.lp, lp);

			const after = await snap();
			assert.equal(after.rA, before.rA + maxA);
			assert.equal(after.rB, before.rB + 10_000_000_000n);
			assert.equal(after.supply, before.supply + lp);
		});

		it("rejects when LP minted is below min_lp", async () => {
			await expectErr(
				program.methods
					.addLiquidity(bn(1_000_000_000n), bn(1_000_000_000n), bn(10_000_000_000_000n))
					.accounts(liqAccounts() as any)
					.rpc(),
				"SlippageExceeded"
			);
		});
	});

	// =====================================================
	describe("swap", () => {
		it("swaps A to B with the expected output", async () => {
			const s = await snap();
			const amountIn = 1_000_000_000n;
			const expected = swapOut(amountIn, s.rA, s.rB, FEE_BPS);
			const userBBefore = await bal(userB);

			await program.methods
				.swap(bn(amountIn), bn(expected), true)
				.accounts(swapAccounts() as any)
				.rpc();

			assert.equal((await bal(userB)) - userBBefore, expected);
			const after = await snap();
			assert.equal(after.rA, s.rA + amountIn); // full input, fee stays in vault
			assert.equal(after.rB, s.rB - expected);
			assert.isTrue(after.rA * after.rB >= s.rA * s.rB, "k must not decrease");
		});

		it("swaps B to A with the expected output", async () => {
			const s = await snap();
			const amountIn = 500_000_000n;
			const expected = swapOut(amountIn, s.rB, s.rA, FEE_BPS);
			const userABefore = await bal(userA);

			await program.methods
				.swap(bn(amountIn), bn(expected), false)
				.accounts(swapAccounts() as any)
				.rpc();

			assert.equal((await bal(userA)) - userABefore, expected);
			const after = await snap();
			assert.equal(after.rB, s.rB + amountIn);
			assert.equal(after.rA, s.rA - expected);
			assert.isTrue(after.rA * after.rB >= s.rA * s.rB, "k must not decrease");
		});

		it("rejects when output is below min_out", async () => {
			const s = await snap();
			const amountIn = 1_000_000_000n;
			const expected = swapOut(amountIn, s.rA, s.rB, FEE_BPS);
			await expectErr(
				program.methods.swap(bn(amountIn), bn(expected + 1n), true).accounts(swapAccounts() as any).rpc(),
				"SlippageExceeded"
			);
		});

		it("rejects zero input", async () => {
			await expectErr(
				program.methods.swap(bn(0n), bn(0n), true).accounts(swapAccounts() as any).rpc(),
				"ZeroAmount"
			);
		});

		it("rejects a vault that is not the pool's vault", async () => {
			const fake = await createAccount(conn, payer, mintA, payer.publicKey, Keypair.generate());
			await expectErr(
				program.methods
					.swap(bn(1_000_000n), bn(0n), true)
					.accounts({ ...swapAccounts(), vaultA: fake } as any)
					.rpc(),
				"ConstraintHasOne"
			);
		});
	});

	// =====================================================
	describe("remove_liquidity", () => {
		it("burns part of the LP and pays out pro-rata", async () => {
			const s = await snap();
			const lpBal = await bal(userLp);
			const burn = lpBal / 4n;
			const outA = (burn * s.rA) / s.supply;
			const outB = (burn * s.rB) / s.supply;
			const uA = await bal(userA);
			const uB = await bal(userB);

			await program.methods
				.removeLiquidity(bn(burn), bn(outA), bn(outB))
				.accounts(liqAccounts() as any)
				.rpc();

			assert.equal((await bal(userA)) - uA, outA);
			assert.equal((await bal(userB)) - uB, outB);
			assert.equal(await bal(userLp), lpBal - burn);

			const after = await snap();
			assert.equal(after.supply, s.supply - burn);
			assert.equal(after.rA, s.rA - outA);
			assert.equal(after.rB, s.rB - outB);
		});

		it("rejects when payout is below the minimums", async () => {
			const s = await snap();
			const burn = (await bal(userLp)) / 10n;
			const outA = (burn * s.rA) / s.supply;
			const outB = (burn * s.rB) / s.supply;
			await expectErr(
				program.methods
					.removeLiquidity(bn(burn), bn(outA + 1n), bn(outB))
					.accounts(liqAccounts() as any)
					.rpc(),
				"SlippageExceeded"
			);
		});

		it("rejects zero LP", async () => {
			await expectErr(
				program.methods.removeLiquidity(bn(0n), bn(0n), bn(0n)).accounts(liqAccounts() as any).rpc(),
				"ZeroAmount"
			);
		});

		it("removing everything leaves only the locked 1,000 LP units", async () => {
			const s = await snap();
			const all = await bal(userLp);
			const outA = (all * s.rA) / s.supply;
			const outB = (all * s.rB) / s.supply;

			await program.methods
				.removeLiquidity(bn(all), bn(outA), bn(outB))
				.accounts(liqAccounts() as any)
				.rpc();

			const after = await snap();
			assert.equal(after.supply, MIN_LIQ);
			assert.equal(await bal(userLp), 0n);
			assert.isTrue(after.rA > 0n && after.rB > 0n, "locked dust stays in vaults");
		});
	});
});