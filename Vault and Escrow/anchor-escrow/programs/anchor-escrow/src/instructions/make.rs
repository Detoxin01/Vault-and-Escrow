use crate::{constants::ESCROW_SEED, error::EscrowError, state::Escrow};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = maker,
        space = Escrow::DISCRIMINATOR.len() + Escrow::INIT_SPACE,
        seeds = [ESCROW_SEED, maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    /// Token account for `mint_a` owned by the `escrow` PDA — holds the deposit.
    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Make<'info> {
    pub fn init_escrow(
        &mut self,
        seed: u64,
        receive: u64,
        unlock_delay: i64,
        duration: i64,
        verifier: Option<Pubkey>,
        bumps: &MakeBumps,
    ) -> Result<()> {
        require!(receive > 0, EscrowError::InvalidAmount);
        require_keys_neq!(self.mint_a.key(), self.mint_b.key(), EscrowError::SameMint);

        let now = Clock::get()?.unix_timestamp;
        let (unlock_at, expires_at) =
            Escrow::compute_window(now, unlock_delay, duration, verifier.is_some())?;

        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            receive,
            created_at: now,
            unlock_at,
            expires_at,
            delivery_confirmed: verifier.is_none(),
            confirmed_at: 0,
            verifier,
            bump: bumps.escrow,
        });
        Ok(())
    }

    pub fn deposit(&mut self, deposit: u64) -> Result<()> {
        require!(deposit > 0, EscrowError::InvalidAmount);

        let cpi_ctx = CpiContext::new(
            self.token_program.key(),
            TransferChecked {
                from: self.maker_ata_a.to_account_info(),
                mint: self.mint_a.to_account_info(),
                to: self.vault.to_account_info(),
                authority: self.maker.to_account_info(),
            },
        );
        transfer_checked(cpi_ctx, deposit, self.mint_a.decimals)
    }
}
