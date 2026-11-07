use crate::{constants::ESCROW_SEED, error::EscrowError, state::Escrow};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    /// Receives `mint_b` payment and rent from the closed `vault` & `escrow`.
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    /// Optional inline co-signer when `escrow.verifier` is set and
    /// `confirm_delivery` has not been called beforehand.
    pub verifier: Option<Signer<'info>>,

    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
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

impl<'info> Take<'info> {
    /// Enforces `Clock` time bounds, condition (delivery confirmation or
    /// inline verifier co-signature), and `expected_receive` slippage guard.
    pub fn check(&self, expected_receive: u64) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(!self.escrow.is_locked(now), EscrowError::EscrowLocked);
        require!(!self.escrow.is_expired(now), EscrowError::EscrowExpired);

        if let Some(expected_verifier) = self.escrow.verifier {
            if !self.escrow.delivery_confirmed {
                let signer = self
                    .verifier
                    .as_ref()
                    .ok_or(error!(EscrowError::ConditionNotMet))?;
                require_keys_eq!(
                    signer.key(),
                    expected_verifier,
                    EscrowError::UnauthorizedVerifier
                );
            }
        }

        require_eq!(
            self.escrow.receive,
            expected_receive,
            EscrowError::ReceiveMismatch
        );
        Ok(())
    }

    /// Taker -> maker: `receive` of `mint_b`.
    pub fn pay_maker(&mut self) -> Result<()> {
        let cpi_ctx = CpiContext::new(
            self.token_program.key(),
            TransferChecked {
                from: self.taker_ata_b.to_account_info(),
                mint: self.mint_b.to_account_info(),
                to: self.maker_ata_b.to_account_info(),
                authority: self.taker.to_account_info(),
            },
        );
        transfer_checked(cpi_ctx, self.escrow.receive, self.mint_b.decimals)
    }

    /// Vault -> taker: full deposit of `mint_a`, then close the vault.
    pub fn withdraw_and_close_vault(&mut self) -> Result<()> {
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
                to: self.taker_ata_a.to_account_info(),
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
