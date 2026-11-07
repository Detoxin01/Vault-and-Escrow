use crate::{constants::MARKET_SEED, error::MarketError, state::Market};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    burn, transfer_checked, Burn, Mint, TokenAccount, TokenInterface, TransferChecked,
};

/// Burn position tokens, get base tokens back 1:1 — only if the market has
/// enough idle liquidity.
#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub user: Signer<'info>,

    #[account(
        mut,
        token::mint = base_mint,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_base: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = position_mint,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_position: InterfaceAccount<'info, TokenAccount>,

    #[account(
        has_one = base_mint,
        has_one = position_mint,
        seeds = [MARKET_SEED, market.authority.as_ref(), base_mint.key().as_ref()],
        bump = market.bump,
    )]
    pub market: Account<'info, Market>,

    pub base_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub position_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = market,
        associated_token::token_program = token_program,
    )]
    pub liquidity: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, MarketError::InvalidAmount);
        require!(
            self.liquidity.amount >= amount,
            MarketError::InsufficientLiquidity
        );

        burn(
            CpiContext::new(
                self.token_program.key(),
                Burn {
                    mint: self.position_mint.to_account_info(),
                    from: self.user_position.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            amount,
        )?;

        let authority = self.market.authority;
        let base_mint = self.base_mint.key();
        let bump = [self.market.bump];
        let seeds: &[&[&[u8]]] = &[&[MARKET_SEED, authority.as_ref(), base_mint.as_ref(), &bump]];

        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.key(),
                TransferChecked {
                    from: self.liquidity.to_account_info(),
                    mint: self.base_mint.to_account_info(),
                    to: self.user_base.to_account_info(),
                    authority: self.market.to_account_info(),
                },
                seeds,
            ),
            amount,
            self.base_mint.decimals,
        )
    }
}
