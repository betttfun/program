use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use anchor_lang::solana_program::sysvar::SysvarId;

pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";
pub const BET_VAULT_SEED: &[u8] = b"bet_vault";
pub const CBET_VAULT_SEED: &[u8] = b"chiefsbet_vault";
pub const EBET_VAULT_SEED: &[u8] = b"eaglesbet_vault";

pub const TOKEN_DECIMALS: u32 = 6;
pub const INITIAL_TOKEN_AMOUNT: u64 = 1_000_000_000;
pub const INITIAL_TOKEN_SUPPLY: u64 = INITIAL_TOKEN_AMOUNT * 10u64.pow(TOKEN_DECIMALS);
pub const BET_DEADLINE: i64 = 1739143800;

pub const WITHDRAWAL_ALLOWED_AFTER: i64 = BET_DEADLINE + 1800;

pub const ADMIN_WALLET: Pubkey = Pubkey::new_from_array(
    pubkey!("4grLJhLZZAXKuHcLa6BSH5UVHQdTLdoW6tRY1Qou5nz4").to_bytes()
);

declare_id!("Aq8F58jNwm3g2at6bEA9fBjRJ17EvLvF6fuSsutZ5XW9");

#[program]
pub mod bet_token {
    use super::*;

    pub fn initialize_bet_vault(ctx: Context<InitializeBetVault>) -> Result<()> {
        Ok(())
    }

    pub fn initialize_cbet_vault(ctx: Context<InitializeCBetVault>) -> Result<()> {
        let amount: u64 = INITIAL_TOKEN_SUPPLY;

        let cpi_accounts = Transfer {
            from: ctx.accounts.payer_cbet_token_account.to_account_info(),
            to: ctx.accounts.cbet_vault.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        msg!("CBET vault initialized");

        Ok(())
    }

    pub fn initialize_ebet_vault(ctx: Context<InitializeEBetVault>) -> Result<()> {
        let amount: u64 = INITIAL_TOKEN_SUPPLY;

        let cpi_accounts = Transfer {
            from: ctx.accounts.payer_ebet_token_account.to_account_info(),
            to: ctx.accounts.ebet_vault.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        msg!("Eagles vault initialized");

        Ok(())
    }

    pub fn place_bet(ctx: Context<PlaceBet>, bet_type: u8, amount: u64) -> Result<()> {
        if ctx.accounts.clock.unix_timestamp >= BET_DEADLINE {
            return Err(ErrorCode::BettingClosed.into());
        }

        if amount == 0 {
            return Err(ErrorCode::InvalidBetAmount.into());
        }

        if bet_type > 1 {
            return Err(ErrorCode::InvalidBetType.into());
        }
    
        let main_transfer_accounts = Transfer {
            from: ctx.accounts.bettor_main_token_account.to_account_info(),
            to: ctx.accounts.bet_vault.to_account_info(),
            authority: ctx.accounts.bettor.to_account_info(),
        };
        let main_transfer_ctx =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), main_transfer_accounts);
        token::transfer(main_transfer_ctx, amount)?;
    
        let (side_vault, side_mint) = match bet_type {
            0 => (&ctx.accounts.ebet_vault, ctx.accounts.ebet_mint.key()),
            1 => (&ctx.accounts.cbet_vault, ctx.accounts.cbet_mint.key()),
            _ => unreachable!()
        };
    
        if ctx.accounts.bettor_side_token_account.mint != side_mint {
            return Err(ErrorCode::InvalidSideTokenAccount.into());
        }
    
        let vault_auth_bump = ctx.bumps.vault_authority;
        let seeds: &[&[u8]] = &[VAULT_AUTHORITY_SEED, &[vault_auth_bump]];
        let signer: &[&[&[u8]]] = &[seeds];
    
        let side_transfer_accounts = Transfer {
            from: side_vault.to_account_info(),
            to: ctx.accounts.bettor_side_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        let side_transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            side_transfer_accounts,
            signer,
        );
        token::transfer(side_transfer_ctx, amount)?;

        msg!("Bet placed on {}", bet_type);
    
        Ok(())
    }
    
    pub fn withdraw_all(ctx: Context<WithdrawAll>) -> Result<()> {
        if ctx.accounts.clock.unix_timestamp < WITHDRAWAL_ALLOWED_AFTER {
            return Err(ErrorCode::WithdrawalNotAllowed.into());
        }

        let vault_auth_bump = ctx.bumps.vault_authority;
        let seeds: &[&[u8]] = &[VAULT_AUTHORITY_SEED, &[vault_auth_bump]];
        let signer: &[&[&[u8]]] = &[seeds];

        let bet_vault_balance = ctx.accounts.bet_vault.amount;
        if bet_vault_balance > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.bet_vault.to_account_info(),
                to: ctx.accounts.authorized_bet_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer,
            );
            token::transfer(cpi_ctx, bet_vault_balance)?;
        }

        let cbet_vault_balance = ctx.accounts.cbet_vault.amount;
        if cbet_vault_balance > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.cbet_vault.to_account_info(),
                to: ctx.accounts.authorized_cbet_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer,
            );
            token::transfer(cpi_ctx, cbet_vault_balance)?;
        }

        let ebet_vault_balance = ctx.accounts.ebet_vault.amount;
        if ebet_vault_balance > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.ebet_vault.to_account_info(),
                to: ctx.accounts.authorized_ebet_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer,
            );
            token::transfer(cpi_ctx, ebet_vault_balance)?;
        }

        msg!("Withdrawal successful");
        
        Ok(())
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Bet amount must be nonzero.")]
    InvalidBetAmount,
    #[msg("The provided token account does not match the required mint for the bet type.")]
    InvalidSideTokenAccount,
    #[msg("Betting period is over. No more bets can be placed.")]
    BettingClosed,
    #[msg("Withdrawal not allowed until after game start + 30 minutes.")]
    WithdrawalNotAllowed,
    #[msg("Invalid bet type. Must be 0 for Eagles or 1 for Chiefs.")]
    InvalidBetType,
}

#[derive(Accounts)]
pub struct InitializeBetVault<'info> {
    #[account(mut, address = ADMIN_WALLET)]
    pub payer: Signer<'info>,

    pub bet_mint: Box<Account<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = payer,
        seeds = [BET_VAULT_SEED, bet_mint.key().as_ref()],
        bump,
        token::mint = bet_mint,
        token::authority = vault_authority,
    )]
    pub bet_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [VAULT_AUTHORITY_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitializeCBetVault<'info> {
    #[account(mut, address = ADMIN_WALLET)]
    pub payer: Signer<'info>,

    pub cbet_mint: Box<Account<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = payer,
        seeds = [CBET_VAULT_SEED, cbet_mint.key().as_ref()],
        bump,
        token::mint = cbet_mint,
        token::authority = vault_authority,
    )]
    pub cbet_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = payer_cbet_token_account.owner == payer.key(),
        constraint = payer_cbet_token_account.mint == cbet_mint.key(),
    )]
    pub payer_cbet_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [VAULT_AUTHORITY_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitializeEBetVault<'info> {
    #[account(mut, address = ADMIN_WALLET)]
    pub payer: Signer<'info>,

    pub ebet_mint: Box<Account<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = payer,
        seeds = [EBET_VAULT_SEED, ebet_mint.key().as_ref()],
        bump,
        token::mint = ebet_mint,
        token::authority = vault_authority,
    )]
    pub ebet_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = payer_ebet_token_account.owner == payer.key(),
        constraint = payer_ebet_token_account.mint == ebet_mint.key(),
    )]
    pub payer_ebet_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [VAULT_AUTHORITY_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub bettor: Signer<'info>,

    #[account(
        mut,
        constraint = bettor_main_token_account.owner == bettor.key(),
        constraint = bettor_main_token_account.mint == bet_mint.key(),
    )]
    pub bettor_main_token_account: Box<Account<'info, TokenAccount>>,

    pub bet_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [BET_VAULT_SEED, bet_mint.key().as_ref()],
        bump,
    )]
    pub bet_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [VAULT_AUTHORITY_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    pub ebet_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = [EBET_VAULT_SEED, ebet_mint.key().as_ref()],
        bump,
    )]
    pub ebet_vault: Box<Account<'info, TokenAccount>>,

    pub cbet_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = [CBET_VAULT_SEED, cbet_mint.key().as_ref()],
        bump,
    )]
    pub cbet_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub bettor_side_token_account: Box<Account<'info, TokenAccount>>,

    #[account(address = Clock::id())]
    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawAll<'info> {
    #[account(mut, address = ADMIN_WALLET)]
    pub authorized_wallet: Signer<'info>,

    #[account(
        mut,
        constraint = authorized_bet_token_account.owner == authorized_wallet.key(),
        constraint = authorized_bet_token_account.mint == bet_mint.key(),
    )]
    pub authorized_bet_token_account: Box<Account<'info, TokenAccount>>,
    pub bet_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = [BET_VAULT_SEED, bet_mint.key().as_ref()],
        bump,
    )]
    pub bet_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = authorized_cbet_token_account.owner == authorized_wallet.key(),
        constraint = authorized_cbet_token_account.mint == cbet_mint.key(),
    )]
    pub authorized_cbet_token_account: Box<Account<'info, TokenAccount>>,
    pub cbet_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = [CBET_VAULT_SEED, cbet_mint.key().as_ref()],
        bump,
    )]
    pub cbet_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = authorized_ebet_token_account.owner == authorized_wallet.key(),
        constraint = authorized_ebet_token_account.mint == ebet_mint.key(),
    )]
    pub authorized_ebet_token_account: Box<Account<'info, TokenAccount>>,
    pub ebet_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = [EBET_VAULT_SEED, ebet_mint.key().as_ref()],
        bump,
    )]
    pub ebet_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [VAULT_AUTHORITY_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(address = Clock::id())]
    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}
