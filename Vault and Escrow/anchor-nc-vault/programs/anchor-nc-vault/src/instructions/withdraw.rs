use crate::{
    constants::VAULT_SEED,
    error::VaultError,
    state::{pro_rata, total_assets, Vault},
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        burn, transfer_checked, Burn, Mint, TokenAccount, TokenInterface, TransferChecked,
    },
};

/// Liquid exit: burn shares for base tokens at NAV. Only works while the
/// vault has enough idle base tokens — otherwise use `redeem_in_kind`.
#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        has_one = base_mint,
        has_one = position_mint,
        has_one = share_mint,
        seeds = [VAULT_SEED, vault.manager.as_ref(), vault.market.as_ref()],
        bump = vault.bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub base_mint: Box<InterfaceAccount<'info, Mint>>,
    pub position_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub share_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = base_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_base: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = share_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_shares: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub idle: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        associated_token::mint = position_mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub vault_position: Box<InterfaceAccount<'info, TokenAccount>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, shares: u64, min_out: u64) -> Result<()> {
        require!(shares > 0, VaultError::InvalidAmount);

        let assets = total_assets(self.idle.amount, self.vault_position.amount)?;
        let out = pro_rata(assets, shares, self.share_mint.supply)?;
        require!(out > 0, VaultError::NothingToRedeem);
        require!(out >= min_out, VaultError::SlippageExceeded);
        require!(self.idle.amount >= out, VaultError::InsufficientLiquidity);

        burn(
            CpiContext::new(
                self.token_program.key(),
                Burn {
                    mint: self.share_mint.to_account_info(),
                    from: self.user_shares.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            shares,
        )?;

        let bump = [self.vault.bump];
        let seeds: &[&[&[u8]]] = &[&[
            VAULT_SEED,
            self.vault.manager.as_ref(),
            self.vault.market.as_ref(),
            &bump,
        ]];
        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.key(),
                TransferChecked {
                    from: self.idle.to_account_info(),
                    mint: self.base_mint.to_account_info(),
                    to: self.user_base.to_account_info(),
                    authority: self.vault.to_account_info(),
                },
                seeds,
            ),
            out,
            self.base_mint.decimals,
        )
    }
}
