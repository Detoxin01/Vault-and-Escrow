use crate::{constants::MARKET_SEED, error::MarketError, state::Market};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked};

/// Market authority takes liquidity out (stands in for borrowers). Used to
/// simulate a fully-utilised market where suppliers cannot withdraw.
#[derive(Accounts)]
pub struct Borrow<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        token::mint = base_mint,
        token::authority = authority,
        token::token_program = token_program,
    )]
    pub authority_base: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        has_one = authority,
        has_one = base_mint,
        seeds = [MARKET_SEED, authority.key().as_ref(), base_mint.key().as_ref()],
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

impl<'info> Borrow<'info> {
    pub fn borrow(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, MarketError::InvalidAmount);
        require!(
            self.liquidity.amount >= amount,
            MarketError::InsufficientLiquidity
        );

        let authority = self.authority.key();
        let base_mint = self.base_mint.key();
        let bump = [self.market.bump];
        let seeds: &[&[&[u8]]] = &[&[MARKET_SEED, authority.as_ref(), base_mint.as_ref(), &bump]];

        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.key(),
                TransferChecked {
                    from: self.liquidity.to_account_info(),
                    mint: self.base_mint.to_account_info(),
                    to: self.authority_base.to_account_info(),
                    authority: self.market.to_account_info(),
                },
                seeds,
            ),
            amount,
            self.base_mint.decimals,
        )?;

        self.market.borrowed = self.market.borrowed.saturating_add(amount);
        Ok(())
    }
}
