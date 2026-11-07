use crate::{constants::VAULT_SEED, error::VaultError, state::Vault};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use mock_market::{program::MockMarket, state::Market};

/// Manager-only: move funds between the vault's idle account and the
/// whitelisted market. Both legs land in vault-owned accounts, so the manager
/// steers the strategy but can never take custody of user funds.
#[derive(Accounts)]
pub struct Rebalance<'info> {
    pub manager: Signer<'info>,

    #[account(
        has_one = manager,
        has_one = market,
        has_one = base_mint,
        has_one = position_mint,
        seeds = [VAULT_SEED, manager.key().as_ref(), market.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub idle: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = position_mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub vault_position: Box<InterfaceAccount<'info, TokenAccount>>,

    pub market: Box<Account<'info, Market>>,

    pub base_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub position_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = market,
        associated_token::token_program = token_program,
    )]
    pub market_liquidity: Box<InterfaceAccount<'info, TokenAccount>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub market_program: Program<'info, MockMarket>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Rebalance<'info> {
    fn market_accounts(&self) -> (Pubkey, mock_market::cpi::accounts::Supply<'info>) {
        (
            self.market_program.key(),
            mock_market::cpi::accounts::Supply {
                user: self.vault.to_account_info(),
                user_base: self.idle.to_account_info(),
                user_position: self.vault_position.to_account_info(),
                market: self.market.to_account_info(),
                base_mint: self.base_mint.to_account_info(),
                position_mint: self.position_mint.to_account_info(),
                liquidity: self.market_liquidity.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }

    /// idle base -> market (vault receives position tokens).
    pub fn allocate(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        let bump = [self.vault.bump];
        let manager = self.vault.manager;
        let market = self.vault.market;
        let seeds: &[&[&[u8]]] = &[&[VAULT_SEED, manager.as_ref(), market.as_ref(), &bump]];

        let (program, accounts) = self.market_accounts();
        mock_market::cpi::supply(CpiContext::new_with_signer(program, accounts, seeds), amount)
    }

    /// market -> idle base (vault burns position tokens). Fails if the market
    /// itself is out of liquidity.
    pub fn deallocate(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        let bump = [self.vault.bump];
        let manager = self.vault.manager;
        let market = self.vault.market;
        let seeds: &[&[&[u8]]] = &[&[VAULT_SEED, manager.as_ref(), market.as_ref(), &bump]];

        let (program, s) = self.market_accounts();
        let accounts = mock_market::cpi::accounts::Withdraw {
            user: s.user,
            user_base: s.user_base,
            user_position: s.user_position,
            market: s.market,
            base_mint: s.base_mint,
            position_mint: s.position_mint,
            liquidity: s.liquidity,
            token_program: s.token_program,
        };
        mock_market::cpi::withdraw(CpiContext::new_with_signer(program, accounts, seeds), amount)
    }
}
