use anchor_lang::{prelude::*, solana_program::pubkey};
use anchor_spl::token::{Mint, Token, TokenAccount};
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod non_custodial_escrow {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, x_amount: u64, y_amount: u64) -> Result<()> {
        // Initialize the fields for the escrow PDA account 
        let escrow = &mut ctx.accounts.escrow;
        escrow.bump = *ctx.bumps.get("escrow").unwrap();
        escrow.authority = ctx.accounts.seller.key();
        escrow.escrowed_x_tokens = ctx.accounts.escrowed_x_tokens.key();
        escrow.y_amount = y_amount; // number of token sellers wants in exchange
        escrow.y_mint = ctx.accounts.y_mint.key(); // token seller wants in exchange

        // Use the spl token account transfer instruction via CPI
        // Transfer seller's x_token to program owned escrow token account
        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(), 
                anchor_spl::token::Transfer {
                    from: ctx.accounts.seller_x_token.to_account_info(), 
                    to: ctx.accounts.escrowed_x_tokens.to_account_info(), 
                    authority: ctx.accounts.seller.to_account_info(), 
                }, 
            ), 
            x_amount, 
        )?;
        Ok(())
    }

    pub fn accept(ctx: Context<Accept>) -> Result<()> {
        // Transfer tokens from ecrowed_x_tokens to buyer_x_tokens
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.escrowed_x_tokens.to_account_info(),
                    to: ctx.accounts.buyer_x_tokens.to_account_info(),
                    authority: ctx.accounts.escrow.to_account_info(),
                },
                &[&["escrow".as_bytes(), ctx.accounts.escrow.authority.as_ref(), &[ctx.accounts.escrow.bump]]],
            ),
            ctx.accounts.escrowed_x_tokens.amount,
        )?;

        // Transfer tokens from buyer_y_tokens to seller_y_tokens
        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.buyer_y_tokens.to_account_info(),
                    to: ctx.accounts.sellers_y_tokens.to_account_info(),
                    authority: ctx.accounts.buyer.to_account_info(),
                },
            ),
            ctx.accounts.escrow.y_amount,
        )?;
        Ok(())
    }

    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        // Transfer x-tokens from escrow x token account to seller account
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.escrowed_x_tokens.to_account_info(),
                    to: ctx.accounts.seller_x_token.to_account_info(),
                    authority: ctx.accounts.escrow.to_account_info(),
                },
                &[&["escrow".as_bytes(), ctx.accounts.seller.key().as_ref(), &[ctx.accounts.escrow.bump]]],
            ),
            ctx.accounts.escrowed_x_tokens.amount,
        )?;

        // Close escrow token account
        anchor_spl::token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::CloseAccount {
                account: ctx.accounts.escrowed_x_tokens.to_account_info(),
                destination: ctx.accounts.seller.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            &[&["escrow".as_bytes(), ctx.accounts.seller.key().as_ref(), &[ctx.accounts.escrow.bump]]],
        ))?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    pub seller: Signer<'info>, // Seller (signer)
    #[account(
        mut, 
        close = seller, constraint = escrow.authority == seller.key(),
        seeds = ["escrow".as_bytes(), escrow.authority.as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>, // Escrow (PDA)
    #[account(mut, constraint = escrowed_x_tokens.key() == escrow.escrowed_x_tokens)]
    pub escrowed_x_tokens: Account<'info, TokenAccount>, // Escrow x-token-account
    #[account(
        mut, 
        constraint = seller_x_token.mint == escrowed_x_tokens.mint, 
        constraint = seller_x_token.owner == seller.key()
    )]
    pub seller_x_token: Account<'info, TokenAccount>, // Seller x-token-account
    pub token_program: Program<'info, Token>, // Token program
}

#[derive(Accounts)]
pub struct Accept<'info> {
    pub buyer: Signer<'info>, // signer (buyer)
    #[account(
        mut, 
        seeds = ["escrow".as_bytes(), escrow.authority.as_ref()], 
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>, // Escrow Account (pda)
    #[account(mut, constraint = escrowed_x_tokens.key() == escrow.escrowed_x_tokens)]
    pub escrowed_x_tokens: Account<'info, TokenAccount>, // Escrow x-token account 
    #[account(mut, constraint = sellers_y_tokens.mint == escrow.y_mint)]
    pub sellers_y_tokens: Account<'info, TokenAccount>, // seller y-token account 
    #[account(mut, constraint = buyer_x_tokens.mint == escrowed_x_tokens.mint)]
    pub buyer_x_tokens: Account<'info, TokenAccount>, // buyer x-token account 
    #[account(
        mut,
        constraint = buyer_y_tokens.mint == escrow.y_mint,
        constraint = buyer_y_tokens.owner == buyer.key()
    )]
    pub buyer_y_tokens: Account<'info, TokenAccount>, // buyer y-token account
    pub token_program: Program<'info, Token>, // token program
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)] 
    seller: Signer<'info>, // Signer
    x_mint: Account<'info, Mint>, // x-mint account
    y_mint: Account<'info, Mint>, // y-mint account
    #[account(mut, constraint = seller_x_token.mint == x_mint.key() && seller_x_token.owner == seller.key())] 
    seller_x_token: Account<'info, TokenAccount>, // user x-token account (mut)
    #[account(
        init, 
        payer = seller, 
        space=Escrow::LEN, 
        seeds=["escrow".as_bytes(), seller.key().as_ref()],
        bump,
    )]
    pub escrow: Account<'info, Escrow>,// PDA (Escrow Account)
    #[account(
        init, 
        payer=seller, 
        token::mint = x_mint, 
        token::authority = escrow, 
    )]
    escrowed_x_tokens: Account<'info, TokenAccount>, // Escrow x-token account
    token_program: Program<'info, Token>, // Token Program
    rent: Sysvar<'info, Rent>, // rent 
    system_program: Program<'info, System> // system program
}

#[account]
pub struct Escrow {
    authority: Pubkey, 
    bump: u8, 
    escrowed_x_tokens: Pubkey,
    y_mint: Pubkey, 
    y_amount: u64
}

impl Escrow {
    pub const LEN: usize = 
    8 + // Discriminator 
    32 + // pubkey
    1 + // Bump (u8)
    32 + // pubkey
    32 + // pubkey
    8; // u64
}
