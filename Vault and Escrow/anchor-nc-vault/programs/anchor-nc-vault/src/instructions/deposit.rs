use crate::{
    constants::VAULT_SEED,
    error::VaultError,
    state::{shares_for_deposit, total_assets, Vault},
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
    },
};

#[derive(Accounts)]
pub struct Deposit<'info> {
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
        associated_token::mint = base_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_base: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
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

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64, min_shares: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);

        // Price shares *before* the deposit lands.
        let assets = total_assets(self.idle.amount, self.vault_position.amount)?;
        let shares = shares_for_deposit(amount, self.share_mint.supply, assets)?;
        require!(shares > 0, VaultError::ZeroShares);
        require!(shares >= min_shares, VaultError::SlippageExceeded);

        transfer_checked(
            CpiContext::new(
                self.token_program.key(),
                TransferChecked {
                    from: self.user_base.to_account_info(),
                    mint: self.base_mint.to_account_info(),
                    to: self.idle.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            amount,
            self.base_mint.decimals,
        )?;

        let bump = [self.vault.bump];
        let seeds: &[&[&[u8]]] = &[&[
            VAULT_SEED,
            self.vault.manager.as_ref(),
            self.vault.market.as_ref(),
            &bump,
        ]];
        mint_to(
            CpiContext::new_with_signer(
                self.token_program.key(),
                MintTo {
                    mint: self.share_mint.to_account_info(),
                    to: self.user_shares.to_account_info(),
                    authority: self.vault.to_account_info(),
                },
                seeds,
            ),
            shares,
        )
    }
}
