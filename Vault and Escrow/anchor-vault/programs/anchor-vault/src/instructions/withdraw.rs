use crate::{
    constants::{STATE_SEED, VAULT_SEED},
    error::VaultError,
    state::VaultState,
};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [VAULT_SEED, user.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);

        // Keep the rent-exempt reserve in the vault; only `close` may drain it.
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.data_len());
        let available = self.vault.lamports().saturating_sub(rent_exempt);
        require!(amount <= available, VaultError::InsufficientFunds);

        // The vault is a PDA, so the program signs for it with its seeds.
        let user_key = self.user.key();
        let bump = [self.vault_state.vault_bump];
        let signer_seeds: &[&[&[u8]]] = &[&[VAULT_SEED, user_key.as_ref(), &bump]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.system_program.key(),
            Transfer {
                from: self.vault.to_account_info(),
                to: self.user.to_account_info(),
            },
            signer_seeds,
        );
        transfer(cpi_ctx, amount)
    }
}
