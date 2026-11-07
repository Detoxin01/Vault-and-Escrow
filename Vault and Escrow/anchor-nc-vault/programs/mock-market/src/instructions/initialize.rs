use crate::{
    constants::{MARKET_SEED, POSITION_MINT_SEED},
    state::Market,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub base_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = Market::DISCRIMINATOR.len() + Market::INIT_SPACE,
        seeds = [MARKET_SEED, authority.key().as_ref(), base_mint.key().as_ref()],
        bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = authority,
        seeds = [POSITION_MINT_SEED, market.key().as_ref()],
        bump,
        mint::decimals = base_mint.decimals,
        mint::authority = market,
        mint::token_program = token_program,
    )]
    pub position_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = base_mint,
        associated_token::authority = market,
        associated_token::token_program = token_program,
    )]
    pub liquidity: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        self.market.set_inner(Market {
            authority: self.authority.key(),
            base_mint: self.base_mint.key(),
            position_mint: self.position_mint.key(),
            borrowed: 0,
            bump: bumps.market,
            position_mint_bump: bumps.position_mint,
        });
        Ok(())
    }
}
