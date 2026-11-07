use crate::{
    constants::{STATE_SEED, VAULT_SEED},
    state::VaultState,
};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        seeds = [STATE_SEED, user.key().as_ref()],
        bump,
        space = VaultState::DISCRIMINATOR.len() + VaultState::INIT_SPACE,
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [VAULT_SEED, user.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        // Fund the vault with the rent-exempt minimum so the account exists
        // on-chain and can never be garbage-collected by the runtime.
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.data_len());

        let cpi_ctx = CpiContext::new(
            self.system_program.key(),
            Transfer {
                from: self.user.to_account_info(),
                to: self.vault.to_account_info(),
            },
        );
        transfer(cpi_ctx, rent_exempt)?;

        self.vault_state.set_inner(VaultState {
            vault_bump: bumps.vault,
            state_bump: bumps.vault_state,
        });

        Ok(())
    }
}
