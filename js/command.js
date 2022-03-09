const anchor = require('@project-serum/anchor');
const serumCmn = require("@project-serum/common");
const { TOKEN_PROGRAM_ID, Token } = require("@solana/spl-token");
const TokenInstructions = require("@project-serum/serum").TokenInstructions;
const fs = require('fs');

const path = require('path');
const os = require("os");

const idl = JSON.parse(fs.readFileSync(path.resolve(__dirname, '../target/idl/random_prize_game.json')));
const programID = new anchor.web3.PublicKey(idl.metadata.address);

const walletKeyData = JSON.parse(fs.readFileSync(os.homedir() + '/.config/solana/id.json'));
const walletKeypair = anchor.web3.Keypair.fromSecretKey(new Uint8Array(walletKeyData));
const wallet = new anchor.Wallet(walletKeypair);

let ANCHOR_PROVIDER_URL = 'https://api.mainnet-beta.solana.com';

const argv = process.argv;
let values = [];
for(var i = 3;i < argv.length; i ++) {
    if(argv[i].indexOf('--') == -1) {
        values.push(argv[i]);
    }
}

let rewardMint = new anchor.web3.PublicKey('HuMJHQL3UbiECz8ZB7aAWeEG9Nn3WrHmkgwgNpkWYL77')
if(argv.indexOf('--env') > -1) {
    const env = argv[argv.indexOf('--env') + 1];
    if(env == 'devnet') {
        ANCHOR_PROVIDER_URL = 'https://api.devnet.solana.com';
        rewardMint = new anchor.web3.PublicKey('HuMJHQL3UbiECz8ZB7aAWeEG9Nn3WrHmkgwgNpkWYL77')
    } else if(env == 'localnet') {
        ANCHOR_PROVIDER_URL = 'http://localhost:8899';
    }
}

const connection = new anchor.web3.Connection(ANCHOR_PROVIDER_URL);

function getProvider() {
  const provider = new anchor.Provider(
      connection, wallet, { preflightCommitment: "processed" },
  );
  return provider;
};
const provider = getProvider();
let program = new anchor.Program(idl, programID, provider);
const poolRawData = fs.readFileSync('json/pool.json');
let poolKeypair = anchor.web3.Keypair.fromSecretKey(new Uint8Array(JSON.parse(poolRawData)));
let poolPubkey = poolKeypair.publicKey;
let rewardMintObject = new Token(provider.connection, rewardMint, TOKEN_PROGRAM_ID, provider.wallet.payer);

async function getPoolSigner() {
  return anchor.web3.PublicKey.findProgramAddress(
        [
          poolPubkey.toBuffer(),
        ],
        program.programId
    );
}

async function getSolVaultKey() {
  return anchor.web3.PublicKey.findProgramAddress(
        [
          poolPubkey.toBuffer(),
          Buffer.from("sol_vault")
        ],
        program.programId
    );
}

async function getPrizeKey() {
  return anchor.web3.PublicKey.findProgramAddress(
        [
          poolPubkey.toBuffer(),
          Buffer.from("prize")
        ],
        program.programId
    );
}

const initializePool = async () => {
  const [poolSigner, nonce] = await getPoolSigner(poolPubkey, program)
    const [vault, vaultNonce] = await getSolVaultKey(poolPubkey, program)
    const [prize, prizeNonce] = await getPrizeKey(poolPubkey, program)

    rewardMintObject = new Token(provider.connection, rewardMint, TOKEN_PROGRAM_ID, provider.wallet.payer);
    
    let rewardPoolVault = await rewardMintObject.createAccount(poolSigner);

    const tx = await program.rpc.initialize(nonce, vaultNonce, prizeNonce, {
      accounts: {
        authority: provider.wallet.publicKey,
        pool: poolPubkey,
        poolSigner: poolSigner,
        solVault: vault,
        prize: prize,
        rewardMint: rewardMint,
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
    console.log("Successfully initialized!");
}

const setPrizeType = async () => {
    if (!values[0] || values[1]) {
        console.log("Invalid argument. yarn set_prize_type <PRIZE_TYPE> <PROBABILITY>");
        return;
    }

    const prizeType = parseInt(values[0]);
    const probabilty = parseInt(values[1]) * 1000000;
    if(isNaN(prizeType) || prizeType > 255) {
        console.log("Prize type can be one number(1~255)");
        return;
    }

    if(isNaN(probabilty)) {
        console.log('probabilty should be number');
        return;
    }

    const tx = await program.rpc.setPrizeType(prizeType, new anchor.BN(probabilty), {
        accounts: {
            pool: poolPubkey,
            owner: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
        }
    })
}

const addPrize0 = async () => {
    if (!values[0]) {
        console.log("Invalid argument. yarn add_prize0 <REWARD_AMOUNT>");
        return;
    }
    const amount = new anchor.BN(parseInt(values[0]));
    const [prize, prizeNonce] = await getPrizeKey(poolPubkey, program)
    const [vault, vaultNonce] = await getSolVaultKey(poolPubkey, program)
    const tx = await program.rpc.addPrize0(amount, {
        accounts: {
            prize,
            pool: poolPubkey,
            solVault: vault,
            depositor: provider.wallet.publicKey,
            owner: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId
        }
    })
}

const addPrize1 = async () => {
    if (!values[0]) {
        console.log("Invalid argument. yarn add_prize1 <REWARD_AMOUNT>");
        return;
    }
    const amount = new anchor.BN(parseInt(values[0]));
    const [prize, prizeNonce] = await getPrizeKey(poolPubkey, program)
    const [poolSigner, nonce] = await getPoolSigner(poolPubkey, program)
    const poolObject = await program.account.pool.fetch(poolPubkey);
    const tx = await program.rpc.addPrize0(amount, {
        accounts: {
            prize,
            pool: poolPubkey,
            poolSigner,
            rewardVault: poolObject.rewardVault,
            depositor: provider.wallet.publicKey,
            owner: provider.wallet.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId
        }
    })
}

const addPrize2 = async () => {
    if (!values[0]) {
        console.log("Invalid argument. yarn add_prize2 <NFT_MINT_ADDRESS>");
        return;
    }
    const nftMintPubkey = new anchor.web3.PublicKey(values[0]);
    const nftMintObject = new Token(provider.connection, nftMintPubkey, TOKEN_PROGRAM_ID, provider.wallet.payer);
    let nftVault = await nftMintObject.createAccount(poolSigner);

    let nftAccountInfo = await nftMintObject.getOrCreateAssociatedAccountInfo(provider.wallet.publicKey);
    let nftFrom = nftAccountInfo.address;

    const [prize, prizeNonce] = await getPrizeKey(poolPubkey, program)
    const [poolSigner, nonce] = await getPoolSigner(poolPubkey, program)
    const poolObject = await program.account.pool.fetch(poolPubkey);
    const tx = await program.rpc.addPrize0(amount, {
        accounts: {
            prize,
            pool: poolPubkey,
            poolSigner,
            nftVault,
            nftFrom,
            owner: provider.wallet.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId
        }
    })
}

console.log("Program ID: ", programID.toString());
console.log("Wallet: ", provider.wallet.publicKey.toString());

const commandID = argv.indexOf('--command_id=1') > -1 ? 1 : 
                    argv.indexOf('--command_id=2') > -1 ? 2 : 
                    argv.indexOf('--command_id=3') > -1 ? 3 :
                    argv.indexOf('--command_id=4') > -1 ? 4 :
                    argv.indexOf('--command_id=5') > -1 ? 5 :
                    argv.indexOf('--command_id=6') > -1 ? 6 :
                    argv.indexOf('--command_id=7') > -1 ? 7 :
                    argv.indexOf('--command_id=8') > -1 ? 8 : -1;
switch(commandID) {
    case 1:
        initializePool();
        break;
    default:
        console.log('Unrecognized command');
        break;
}