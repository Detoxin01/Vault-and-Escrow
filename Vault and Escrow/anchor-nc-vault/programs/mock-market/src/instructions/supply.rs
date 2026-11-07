use crate::{constants::MARKET_SEED, error::MarketError, state::Market};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

/// Deposit base tokens, receive position tokens 1:1. `user` may be a PDA
/// signing via CPI (e.g. the non-custodial vault).
#[derive(Accounts)]
pub struct Supply<'info> {
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

impl<'info> Supply<'info> {
    pub fn supply(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, MarketError::InvalidAmount);

        transfer_checked(
            CpiContext::new(
                self.token_program.key(),
                TransferChecked {
                    from: self.user_base.to_account_info(),
                    mint: self.base_mint.to_account_info(),
                    to: self.liquidity.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            amount,
            self.base_mint.decimals,
        )?;

        let authority = self.market.authority;
        let base_mint = self.base_mint.key();
        let bump = [self.market.bump];
        let seeds: &[&[&[u8]]] = &[&[MARKET_SEED, authority.as_ref(), base_mint.as_ref(), &bump]];

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.key(),
                MintTo {
                    mint: self.position_mint.to_account_info(),
                    to: self.user_position.to_account_info(),
                    authority: self.market.to_account_info(),
                },
                seeds,
            ),
            amount,
        )
    }
}
