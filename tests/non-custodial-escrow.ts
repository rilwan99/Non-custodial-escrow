import * as anchor from "@project-serum/anchor";
import * as splToken from "@solana/spl-token";
import { Program } from "@project-serum/anchor";
import { NonCustodialEscrow } from "../target/types/non_custodial_escrow";
import { LAMPORTS_PER_SOL, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";

describe("NonCustodialEscrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace
    .NonCustodialEscrow as Program<NonCustodialEscrow>;

  const seller = provider.wallet.publicKey; // anchor.web3.Keypair.generate();
  const payer = (provider.wallet as NodeWallet).payer;
  const authority = anchor.web3.Keypair.generate();
  const buyer = anchor.web3.Keypair.generate();
  const escrowedXTokens = anchor.web3.Keypair.generate();
  let x_mint;
  let y_mint;
  let sellers_x_token;
  let sellers_y_token;
  let buyer_x_token;
  let buyer_y_token;
  let escrow: anchor.web3.PublicKey;

  before(async () => {
    await provider.connection.requestAirdrop(
      buyer.publicKey,
      1 * LAMPORTS_PER_SOL
    );
    // Derive escrow address
    [escrow] = await anchor.web3.PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("escrow"), seller.toBuffer()],
      program.programId
    );
    x_mint = await splToken.createMint(
      provider.connection,
      payer,
      authority.publicKey,
      null,
      6
    );
    y_mint = await splToken.createMint(
      provider.connection,
      payer,
      authority.publicKey,
      null,
      6
    );
    console.log("Mint Accounts Initialized---------");

    // seller's x and y token account
    sellers_x_token = await splToken.createAccount(
      provider.connection,
      payer,
      x_mint,
      seller
    );

    await splToken.mintTo(
      provider.connection,
      payer,
      x_mint,
      sellers_x_token,
      authority,
      100000000000,
      []
    );

    sellers_y_token = await splToken.createAccount(
      provider.connection,
      payer,
      y_mint,
      seller
    );
    console.log("Seller Token Accounts Initialized------");

    // buyer's x and y token account
    buyer_x_token = await splToken.createAccount(
      provider.connection,
      payer,
      x_mint,
      buyer.publicKey
    );

    buyer_y_token = await splToken.createAccount(
      provider.connection,
      payer,
      y_mint,
      buyer.publicKey
    );

    await splToken.mintTo(
      provider.connection,
      payer,
      y_mint,
      buyer_y_token,
      authority,
      100000000000,
      []
    );
    console.log("Buyer Token Accounts Initialized-------------");
  });

  it("Initialize escrow", async () => {
    const x_amount = new anchor.BN(40);
    const y_amount = new anchor.BN(40);
    const tx = await program.methods
      .initialize(x_amount, y_amount)
      .accounts({
        seller: seller,
        xMint: x_mint,
        yMint: y_mint,
        sellerXToken: sellers_x_token,
        escrow: escrow,
        escrowedXTokens: escrowedXTokens.publicKey,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([escrowedXTokens])
      .rpc();
  });

  it("Execute the trade", async () => {
    const tx = await program.methods
      .accept()
      .accounts({
        buyer: buyer.publicKey,
        escrow: escrow,
        escrowedXTokens: escrowedXTokens.publicKey,
        sellersYTokens: sellers_y_token,
        buyerXTokens: buyer_x_token,
        buyerYTokens: buyer_y_token,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
      })
      .signers([buyer])
      .rpc();
  });

  it("Cancle the trade", async () => {
    const tx = await program.methods
      .cancel()
      .accounts({
        seller: seller,
        escrow: escrow,
        escrowedXTokens: escrowedXTokens.publicKey,
        sellerXToken: sellers_x_token,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
      })
      .rpc();
  });
});
