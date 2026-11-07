use crate::{
    constants::VAULT_SEED,
    error::VaultError,
    state::{pro_rata, Vault},
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        burn, transfer_checked, Burn, Mint, TokenAccount, TokenInterface, TransferChecked,
    },
};

/// Always-available exit: burn shares and receive your exact pro-rata slice
/// of *every* asset the vault holds — idle base tokens **and** the market
/// position tokens. Needs no liquidity, no oracle and no manager approval,
/// so users can exit at any time even if the vault is 100% deployed and the
/// market itself is illiquid. The position tokens can later be withdrawn
/// from the market directly by the user.
#[derive(Accounts)]
pub struct RedeemInKind<'info> {
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
        mut,
        associated_token::mint = share_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_shares: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = base_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_base: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = position_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_position: Box<InterfaceAccount<'info, TokenAccount>>,

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

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> RedeemInKind<'info> {
    pub fn redeem_in_kind(&mut self, shares: u64) -> Result<()> {
        require!(shares > 0, VaultError::InvalidAmount);

        let supply = self.share_mint.supply;
        let base_out = pro_rata(self.idle.amount, shares, supply)?;
        let position_out = pro_rata(self.vault_position.amount, shares, supply)?;
        require!(base_out > 0 || position_out > 0, VaultError::NothingToRedeem);

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
        let manager = self.vault.manager;
        let market = self.vault.market;
        let seeds: &[&[&[u8]]] = &[&[VAULT_SEED, manager.as_ref(), market.as_ref(), &bump]];

        if base_out > 0 {
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
                base_out,
                self.base_mint.decimals,
            )?;
        }

        if position_out > 0 {
            transfer_checked(
                CpiContext::new_with_signer(
                    self.token_program.key(),
                    TransferChecked {
                        from: self.vault_position.to_account_info(),
                        mint: self.position_mint.to_account_info(),
                        to: self.user_position.to_account_info(),
                        authority: self.vault.to_account_info(),
                    },
                    seeds,
                ),
                position_out,
                self.position_mint.decimals,
            )?;
        }

        Ok(())
    }
}
