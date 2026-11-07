use crate::{constants::MARKET_SEED, error::MarketError, state::Market};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked};

/// Anyone can put liquidity back into the market.
#[derive(Accounts)]
pub struct Repay<'info> {
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = base_mint,
        token::authority = payer,
        token::token_program = token_program,
    )]
    pub payer_base: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        has_one = base_mint,
        seeds = [MARKET_SEED, market.authority.as_ref(), base_mint.key().as_ref()],
        bump = market.bump,
    )]
    pub market: Account<'info, Market>,

    pub base_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = market,
        associated_token::token_program = token_program,
    )]
    pub liquidity: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Repay<'info> {
    pub fn repay(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, MarketError::InvalidAmount);

        transfer_checked(
            CpiContext::new(
                self.token_program.key(),
                TransferChecked {
                    from: self.payer_base.to_account_info(),
                    mint: self.base_mint.to_account_info(),
                    to: self.liquidity.to_account_info(),
                    authority: self.payer.to_account_info(),
                },
            ),
            amount,
            self.base_mint.decimals,
        )?;

        self.market.borrowed = self.market.borrowed.saturating_sub(amount);
        Ok(())
    }
}
