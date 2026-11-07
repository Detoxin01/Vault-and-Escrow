use crate::{
    constants::{SHARES_SEED, VAULT_SEED},
    state::Vault,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use mock_market::state::Market;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,

    /// The whitelisted market. `Account<Market>` checks it is owned by the
    /// mock-market program; `has_one` ties the mints to it.
    #[account(has_one = base_mint, has_one = position_mint)]
    pub market: Box<Account<'info, Market>>,

    #[account(mint::token_program = token_program)]
    pub base_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_program)]
    pub position_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = manager,
        space = Vault::DISCRIMINATOR.len() + Vault::INIT_SPACE,
        seeds = [VAULT_SEED, manager.key().as_ref(), market.key().as_ref()],
        bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        payer = manager,
        seeds = [SHARES_SEED, vault.key().as_ref()],
        bump,
        mint::decimals = base_mint.decimals,
        mint::authority = vault,
        mint::token_program = token_program,
    )]
    pub share_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = manager,
        associated_token::mint = base_mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub idle: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = manager,
        associated_token::mint = position_mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub vault_position: Box<InterfaceAccount<'info, TokenAccount>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        self.vault.set_inner(Vault {
            manager: self.manager.key(),
            market: self.market.key(),
            base_mint: self.base_mint.key(),
            position_mint: self.position_mint.key(),
            share_mint: self.share_mint.key(),
            bump: bumps.vault,
            share_mint_bump: bumps.share_mint,
        });
        Ok(())
    }
}
