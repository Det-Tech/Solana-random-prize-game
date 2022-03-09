use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program_option::COption, clock};
use anchor_spl::token::{self, TokenAccount, Token, Mint};
use std::convert::Into;
use std::convert::TryInto;
use sha2::{Sha256, Digest};

declare_id!("2u6cS6xYLNYrLeBxvwSUhBLVzEPNHety2UK8AvAXeybj");

#[program]
pub mod random_prize_game {
    use super::*;
    pub fn initialize(
                        ctx: Context<Initialize>,
                        pool_nonce: u8,
                        vault_nonce: u8,
                        prize_nonce: u8,
                        ) -> ProgramResult {

        let pool = &mut ctx.accounts.pool;
        pool.authority = ctx.accounts.authority.key();
        pool.play_tried = 0;
        pool.reward0 = 1;
        pool.reward1 = 1;
        pool.reward2 = 1;
        pool.nonce = pool_nonce;
        pool.vault_nonce = vault_nonce;
        pool.reward_mint = ctx.accounts.reward_mint.key();
        pool.reward_vault = ctx.accounts.reward_vault.key();
        pool.sol_vault = ctx.accounts.sol_vault.key();
        pool.prize_probability = [0, 0, 0];

        let prize = &mut ctx.accounts.prize;
        prize.nonce = prize_nonce;
        prize.authority = ctx.accounts.authority.key();
        prize.prize0 = vec![];
        prize.prize1 = vec![];
        prize.prize2 = vec![];

        Ok(())
    }

    pub fn create_user(ctx: Context<CreateUser>, nonce: u8) -> ProgramResult {
        ctx.accounts.user.owner = ctx.accounts.owner.key();
        ctx.accounts.user.nonce = nonce;
        ctx.accounts.user.win = false;
        Ok(())
    }

    pub fn set_prize_type(
                            ctx: Context<SetPrizeProbability>,
                            prize_type: u8,
                            probability: u64,) -> ProgramResult {
        let pool = &mut ctx.accounts.pool;
        if prize_type > 3 {
            return Err(ErrorCode::MisMatchPrizeType.into());
        }
        pool.prize_probability[prize_type as usize] = probability;

        Ok(())
    }

    pub fn add_prize0(
                        ctx: Context<AddPrize0>,
                        amount: u64,) -> ProgramResult {
        let prize = &mut ctx.accounts.prize;
        prize.prize0.push(amount);

        let ix = anchor_lang::solana_program::system_instruction::transfer(
                                    &ctx.accounts.depositor.key(), 
                                    &ctx.accounts.sol_vault.key(), 
                                    amount);
        anchor_lang::solana_program::program::invoke(&ix, &[
                                                                ctx.accounts.depositor.to_account_info(), 
                                                                ctx.accounts.sol_vault.to_account_info(), 
                                                            ])?;


        Ok(())
    }

    pub fn add_prize1(
                        ctx: Context<AddPrize1>,
                        amount: u64,) -> ProgramResult {
        let prize = &mut ctx.accounts.prize;
        prize.prize1.push(amount);
        
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.depositor.to_account_info(),
                to: ctx.accounts.reward_vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(), //todo use user account as signer
            },
        );
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn add_prize2(ctx: Context<AddPrize2>) -> ProgramResult {
        let prize = &mut ctx.accounts.prize;
        prize.prize2.push(*ctx.accounts.nft_vault.to_account_info().key);
        
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.nft_from.to_account_info(),
                to: ctx.accounts.nft_vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(), //todo use user account as signer
            },
        );
        token::transfer(cpi_ctx, 1 as u64)?;

        Ok(())
    }

    pub fn play(ctx: Context<Play>) -> ProgramResult {
        let ix = anchor_lang::solana_program::system_instruction::transfer(
                                    &ctx.accounts.owner.to_account_info().key(), 
                                    &ctx.accounts.sol_vault.key(), 
                                    100_000_000);
        anchor_lang::solana_program::program::invoke(&ix, &[
                                                                ctx.accounts.owner.to_account_info(), 
                                                                ctx.accounts.sol_vault.to_account_info(), 
                                                            ])?;


        let pool = &mut ctx.accounts.pool;
        let prize = &mut ctx.accounts.prize;
        pool.play_tried = pool.play_tried.checked_add(1).unwrap();

        let user = &mut ctx.accounts.user;

        let current_time: u64 = clock::Clock::get().unwrap().unix_timestamp.try_into().unwrap();
        let mut hasher = Sha256::new();
        hasher.update([
                            ctx.accounts.owner.key.as_ref(),
                            &current_time.to_be_bytes(),
                        ].concat());
        let result = hasher.finalize();
        let mut sum: u64 = 0;
        for i in 0..result.len() {
            sum = sum.checked_add(result[i] as u64).unwrap();
        }

        let current_0_rewards = pool.reward0.checked_mul(1000000).unwrap()
                                            .checked_div(pool.play_tried).unwrap();

        let current_1_rewards = pool.reward1.checked_mul(1000000).unwrap()
                                            .checked_div(pool.play_tried).unwrap();

        let current_2_rewards = pool.reward2.checked_mul(1000000).unwrap()
                                            .checked_div(pool.play_tried).unwrap();
        // type 1
        if sum >= 1500 && sum <= 2500 && current_1_rewards < pool.prize_probability[1]
                || current_1_rewards < pool.prize_probability[1] {
            if prize.prize1.len() > 0 {
                pool.reward1 = pool.reward1.checked_add(1).unwrap();
                let mut prize_index: u64 = 0;
                if sum >= 2000 && sum <= 2500 {
                    prize_index = sum % (prize.prize1.len() as u64);
                }

                user.prize_amount = prize.prize1[prize_index as usize];
                user.prize_token = pool.reward_mint;
                user.prize_type = 1;
                user.win = true;
                prize.prize1.remove(prize_index.try_into().unwrap());
            }
        } 
        // type 0
        else if sum <= 3000 && current_0_rewards < pool.prize_probability[0] 
                || current_0_rewards < pool.prize_probability[0] {
            if prize.prize0.len() > 0 {
                pool.reward0 = pool.reward0.checked_add(1).unwrap();
                let mut prize_index: u64 = 0;
                if sum <= 3000 {
                    prize_index = sum % (prize.prize0.len() as u64);
                }

                user.prize_amount = prize.prize0[prize_index as usize];
                user.prize_token = *ctx.accounts.system_program.key;
                user.prize_type = 0;
                user.win = true;
                prize.prize0.remove(prize_index.try_into().unwrap());
            }
        }
        // type 2
        else if current_2_rewards < pool.prize_probability[2] {
            if prize.prize2.len() > 0 {
                pool.reward2 = pool.reward2.checked_add(1).unwrap();
                let mut prize_index: u64 = 0;
                if sum > 5000 {
                    prize_index = sum % (prize.prize2.len() as u64);
                }

                user.prize_amount = 1 as u64;
                user.prize_type = 2;
                user.prize_token = prize.prize2[prize_index as usize];
                user.win = true;
                prize.prize2.remove(prize_index.try_into().unwrap());
            }
        }

        Ok(())
    }

    pub fn get_prize0(ctx: Context<GetPrize0>) -> ProgramResult {
        let pool = &ctx.accounts.pool;
        let user = &mut ctx.accounts.user;
        if user.win == false {
            return Err(ErrorCode::NotWinner.into());
        }
        let seeds = &[
            pool.to_account_info().key.as_ref(),
            &[pool.nonce],
        ];
        let pool_signer = &[&seeds[..]];
        anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::transfer(
                                    &ctx.accounts.from.key(), 
                                    &ctx.accounts.to.key(), 
                                    user.prize_amount
                                ),
                                &[
                                    ctx.accounts.from.to_account_info(),
                                    ctx.accounts.to.to_account_info(),
                                ],
                                pool_signer,
                            )?;
        user.win = false;
        Ok(())
    }

    pub fn get_prize1(ctx: Context<GetPrize1>) -> ProgramResult {
        let pool = &ctx.accounts.pool;
        let user = &mut ctx.accounts.user;
        if user.win == false {
            return Err(ErrorCode::NotWinner.into());
        }
        let seeds = &[
            pool.to_account_info().key.as_ref(),
            &[pool.nonce],
        ];
        let pool_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.from.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: ctx.accounts.pool_signer.to_account_info(),
            },
            pool_signer,
        );

        token::transfer(cpi_ctx, user.prize_amount)?;

        user.win = false;
        Ok(())
    }

    pub fn get_prize2(ctx: Context<GetPrize2>) -> ProgramResult {
        let pool = &ctx.accounts.pool;
        let user = &mut ctx.accounts.user;
        if user.win == false {
            return Err(ErrorCode::NotWinner.into());
        }
        let seeds = &[
            pool.to_account_info().key.as_ref(),
            &[pool.nonce],
        ];
        let pool_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.from.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: ctx.accounts.pool_signer.to_account_info(),
            },
            pool_signer,
        );

        token::transfer(cpi_ctx, user.prize_amount)?;

        user.win = false;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    authority: Signer<'info>,
    #[account(
        seeds = [
            pool.to_account_info().key.as_ref()
        ],
        bump,
    )]
    /// CHECK: This is contract signer. No need to check
    pool_signer: UncheckedAccount<'info>,
    #[account(zero)]
    pool: Box<Account<'info, Pool>>,
    #[account(
        init,
        payer = authority,
        seeds = [
            pool.to_account_info().key.as_ref(),
            "prize".as_bytes(),
        ],
        bump,
        space = 10240,
    )]
    prize: Box<Account<'info, Prize>>,
    #[account(
        seeds = [
            pool.to_account_info().key.as_ref(),
            "sol_vault".as_bytes(),
        ],
        bump,
    )]
    /// CHECK: This is sol vault. No need to check
    sol_vault: UncheckedAccount<'info>,
    reward_mint: Box<Account<'info, Mint>>,
    #[account(
        constraint = reward_vault.mint == reward_mint.key(),
        constraint = reward_vault.owner == pool_signer.key(),
        constraint = reward_vault.close_authority == COption::None,
    )]
    reward_vault: Box<Account<'info, TokenAccount>>,
    // Misc.
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetPrizeProbability<'info>  {
    #[account(
        mut, 
        constraint = pool.authority == *owner.key
    )]
    pool: Box<Account<'info, Pool>>,
    owner: Signer<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddPrize0<'info> {
    #[account(
        mut, 
        constraint = prize.authority == *owner.key
    )]
    prize: Box<Account<'info, Prize>>,
    #[account(
        constraint = prize.authority == *owner.key
    )]
    pool: Box<Account<'info, Pool>>,
    #[account(
        mut,
        seeds = [
            pool.to_account_info().key.as_ref(),
            "sol_vault".as_bytes(),
        ],
        bump,
    )]
    /// CHECK: This is sol vault. No need to check
    sol_vault: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This is depositor address. No need to check
    depositor: AccountInfo<'info>,
    owner: Signer<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddPrize1<'info> {
    #[account(
        mut, 
        constraint = prize.authority == *owner.key
    )]
    prize: Box<Account<'info, Prize>>,
    #[account(
        constraint = prize.authority == *owner.key
    )]
    pool: Box<Account<'info, Pool>>,
    #[account(
        seeds = [
            pool.to_account_info().key.as_ref()
        ],
        bump,
    )]
    /// CHECK: This is pool signer. No need to check
    pool_signer: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = reward_vault.mint == pool.reward_mint,
        constraint = reward_vault.owner == pool_signer.key(),
        constraint = reward_vault.close_authority == COption::None,
    )]
    reward_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = depositor.mint == pool.reward_mint
    )]
    depositor: Box<Account<'info, TokenAccount>>,
    owner: Signer<'info>,
    // Misc.
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddPrize2<'info> {
    #[account(
        mut, 
        constraint = prize.authority == *owner.key
    )]
    prize: Box<Account<'info, Prize>>,
    #[account(
        constraint = prize.authority == *owner.key
    )]
    pool: Box<Account<'info, Pool>>,
    #[account(
        seeds = [
            pool.to_account_info().key.as_ref()
        ],
        bump,
    )]
    /// CHECK: This is pool signer. No need to check
    pool_signer: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = nft_vault.owner == pool_signer.key(),
        constraint = nft_vault.mint == nft_from.mint,
    )]
    nft_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    nft_from: Box<Account<'info, TokenAccount>>,
    owner: Signer<'info>,
    // Misc.
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Play<'info> {
    #[account(mut)]
    pool: Box<Account<'info, Pool>>,
    #[account(
        mut,
        seeds = [
            pool.to_account_info().key.as_ref(),
            "prize".as_bytes(),
        ],
        bump,
    )]
    prize: Box<Account<'info, Prize>>,
    #[account(
        mut,
        constraint = user.owner == *owner.key
    )]
    user: Box<Account<'info, User>>,
    #[account(
        mut,
        seeds = [
            pool.to_account_info().key.as_ref(),
            "sol_vault".as_bytes(),
        ],
        bump,
    )]
    /// CHECK: This is sol vault. No need to check
    sol_vault: UncheckedAccount<'info>,
    owner: Signer<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetPrize0<'info> {
    pool: Box<Account<'info, Pool>>,
    #[account(
        seeds = [
            pool.to_account_info().key.as_ref()
        ],
        bump,
    )]
    /// CHECK: This is pool signer. No need to check
    pool_signer: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = user.owner == *owner.key
    )]
    user: Box<Account<'info, User>>,
    #[account(
        mut,
        constraint = from.key() == pool.sol_vault,
    )]
    /// CHECK: This is sol vault. No need to check
    from: AccountInfo<'info>,
    #[account(
        mut,
        constraint = to.key() == owner.key(),
    )]
    /// CHECK: This is wallet address. No need to check
    to: AccountInfo<'info>,
    owner: Signer<'info>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetPrize1<'info> {
    pool: Box<Account<'info, Pool>>,
    #[account(
        seeds = [
            pool.to_account_info().key.as_ref()
        ],
        bump,
    )]
    /// CHECK: This is pool signer. No need to check
    pool_signer: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = user.owner == *owner.key
    )]
    user: Box<Account<'info, User>>,
    #[account(
        mut,
        constraint = from.owner == pool_signer.key(),
        constraint = from.mint == to.mint,
    )]
    from: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = to.owner == owner.key(),
    )]
    to: Box<Account<'info, TokenAccount>>,
    owner: Signer<'info>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetPrize2<'info> {
    pool: Box<Account<'info, Pool>>,
    #[account(
        seeds = [
            pool.to_account_info().key.as_ref()
        ],
        bump,
    )]
    /// CHECK: This is pool signer. No need to check
    pool_signer: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = user.owner == *owner.key
    )]
    user: Box<Account<'info, User>>,
    #[account(
        mut,
        constraint = from.owner == pool_signer.key(),
        constraint = from.mint == to.mint,
    )]
    from: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = to.owner == owner.key(),
    )]
    to: Box<Account<'info, TokenAccount>>,
    owner: Signer<'info>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct CreateUser<'info> {
    pool: Box<Account<'info, Pool>>,
    #[account(
        init,
        payer = owner,
        seeds = [
            owner.key.as_ref(), 
            pool.to_account_info().key.as_ref(),
        ],
        bump,
    )]
    user: Box<Account<'info, User>>,
    #[account(mut)]
    owner: Signer<'info>,
    system_program: Program<'info, System>,
}

#[account]
pub struct Pool {
    pub authority: Pubkey,
    pub nonce: u8,
    pub vault_nonce: u8,
    pub sol_vault: Pubkey,
    pub reward_vault: Pubkey,
    pub reward_mint: Pubkey,
    pub prize_probability: [u64; 3],
    pub play_tried: u64,
    pub reward0: u64,
    pub reward1: u64,
    pub reward2: u64,
}

#[account]
pub struct Prize {
    pub nonce: u8,
    pub id: u8,
    pub authority: Pubkey,
    pub prize0: Vec<u64>,
    pub prize1: Vec<u64>,
    pub prize2: Vec<Pubkey>,
}

#[account]
#[derive(Default)]
pub struct User {
    pub nonce: u8,
    pub prize_amount: u64,
    pub prize_token: Pubkey,
    pub prize_type: u8,
    pub win: bool,
    pub owner: Pubkey
}

#[error]
pub enum ErrorCode {
    #[msg("Mismatch prize type")]
    MisMatchPrizeType,
    #[msg("Fee amount is not enough.")]
    FeeNotEnough,
    #[msg("Tried to get prize with not winner.")]
    NotWinner,
}