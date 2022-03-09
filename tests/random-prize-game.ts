const assert = require("assert");
import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { RandomPrizeGame } from '../target/types/random_prize_game';

async function sendLamports(
    provider,
    destination,
    amount
) {
    const tx = new anchor.web3.Transaction();
    tx.add(
        anchor.web3.SystemProgram.transfer(
            { 
                fromPubkey: provider.wallet.publicKey, 
                lamports: amount, 
                toPubkey: destination
            }
        )
    );
    await provider.send(tx);
}

async function createMint(provider, decimals) {
    const mint = await Token.createMint(
        provider.connection,
        provider.wallet.payer,
        provider.wallet.publicKey,
        null,
        decimals,
        TOKEN_PROGRAM_ID
    );
    return mint;
}

async function getPoolSigner(poolPubkey, program) {
  return anchor.web3.PublicKey.findProgramAddress(
        [
          poolPubkey.toBuffer(),
        ],
        program.programId
    );
}

async function getSolVaultKey(poolPubkey, program) {
  return anchor.web3.PublicKey.findProgramAddress(
        [
          poolPubkey.toBuffer(),
          Buffer.from("sol_vault")
        ],
        program.programId
    );
}

async function getPrizeKey(poolPubkey, program) {
  return anchor.web3.PublicKey.findProgramAddress(
        [
          poolPubkey.toBuffer(),
          Buffer.from("prize")
        ],
        program.programId
    );
}

describe('random-prize-game', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.RandomPrizeGame as Program<RandomPrizeGame>;
  const provider = anchor.getProvider();
  let poolKeypair = anchor.web3.Keypair.generate();
  let poolPubkey = poolKeypair.publicKey;
  let rewardMint, rewardMintObject, rewardMintAccount, rewardPoolVault;
  let user1 = anchor.web3.Keypair.generate();

  it('Is initialized!', async () => {
    const [poolSigner, nonce] = await getPoolSigner(poolPubkey, program)
    const [vault, vaultNonce] = await getSolVaultKey(poolPubkey, program)
    const [prize, prizeNonce] = await getPrizeKey(poolPubkey, program)

    rewardMint = await createMint(provider, 9);
    rewardMintObject = new Token(provider.connection, rewardMint.publicKey, TOKEN_PROGRAM_ID, provider.wallet.payer);
    
    rewardMintAccount = await rewardMintObject.createAssociatedTokenAccount(provider.wallet.publicKey);
    await rewardMintObject.mintTo(rewardMintAccount, provider.wallet.payer, [], 100_000_000_000);


    rewardPoolVault = await rewardMintObject.createAccount(poolSigner);

    
    const tx = await program.rpc.initialize(nonce, vaultNonce, prizeNonce, {
      accounts: {
        authority: provider.wallet.publicKey,
        pool: poolPubkey,
        poolSigner: poolSigner,
        solVault: vault,
        prize: prize,
        rewardMint: rewardMint.publicKey,
        rewardVault: rewardPoolVault,
        owner: provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [poolKeypair, ],
      instructions: [
          await program.account.pool.createInstruction(poolKeypair, ),
      ],
    });
    console.log("Your transaction signature", tx);
  });

  it('create user', async () => {
    await sendLamports(provider, user1.publicKey, anchor.web3.LAMPORTS_PER_SOL);

      let envProvider = anchor.Provider.env();
      envProvider.commitment = 'pending';

      let userProvider = new anchor.Provider(envProvider.connection, new anchor.Wallet(user1), envProvider.opts);
      let envProgram = anchor.workspace.RandomPrizeGame;
      let userProgram = new anchor.Program(envProgram.idl, envProgram.programId, userProvider);

    const [userPubkey, userNonce] = await anchor.web3.PublicKey.findProgramAddress(
        [
          user1.publicKey.toBuffer(),
          poolPubkey.toBuffer()
        ],
        userProgram.programId
      );

    const tx = await userProgram.rpc.createUser(userNonce, {
      accounts: {
        pool: poolPubkey,
        user: userPubkey,
        owner: user1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId
      }
    })
  })
});
