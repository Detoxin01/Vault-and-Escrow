use crate::{constants::ESCROW_SEED, error::EscrowError, state::Escrow};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

/// Returns the deposit to the maker and closes the escrow.
///
/// * Maker: anytime while delivery is NOT confirmed, OR after `expires_at`
///   even if delivery was confirmed (if no taker settled in time).
/// * Anyone (permissionless crank): once `expires_at` has passed. Tokens and
///   rent always go back to the maker, never to the caller.
#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    /// Created (paid by `caller`) if the maker closed their ATA in the meantime.
    #[account(
        init_if_needed,
        payer = caller,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = mint_a,
        seeds = [ESCROW_SEED, maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Refund<'info> {
    pub fn check(&self) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        if self.caller.key() == self.maker.key() {
            if self.escrow.verifier.is_some()
                && self.escrow.delivery_confirmed
                && !self.escrow.is_expired(now)
            {
                return err!(EscrowError::DeliveryConfirmedActive);
            }
        } else {
            require!(self.escrow.is_expired(now), EscrowError::NotExpired);
        }
        Ok(())
    }

    pub fn refund_and_close_vault(&mut self) -> Result<()> {
        let maker_key = self.maker.key();
        let seed_bytes = self.escrow.seed.to_le_bytes();
        let bump = [self.escrow.bump];
        let signer_seeds: &[&[&[u8]]] =
            &[&[ESCROW_SEED, maker_key.as_ref(), seed_bytes.as_ref(), &bump]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.key(),
            TransferChecked {
                from: self.vault.to_account_info(),
                mint: self.mint_a.to_account_info(),
                to: self.maker_ata_a.to_account_info(),
                authority: self.escrow.to_account_info(),
            },
            signer_seeds,
        );
        transfer_checked(cpi_ctx, self.vault.amount, self.mint_a.decimals)?;

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.key(),
            CloseAccount {
                account: self.vault.to_account_info(),
                destination: self.maker.to_account_info(),
                authority: self.escrow.to_account_info(),
            },
            signer_seeds,
        );
        close_account(cpi_ctx)
    }
}
