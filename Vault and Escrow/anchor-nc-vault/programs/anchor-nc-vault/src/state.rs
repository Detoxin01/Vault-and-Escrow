use crate::error::VaultError;
use anchor_lang::prelude::*;

/// A share-based vault over two assets:
///
/// * `idle`     – ATA of `base_mint` owned by the vault (liquid funds)
/// * `position` – ATA of `position_mint` owned by the vault (funds deployed
///   into `market`, represented by the market's 1:1 receipt token)
///
/// Users own the vault through `share_mint` tokens held in their own
/// wallets. Nobody (including the manager) can move vault assets anywhere
/// except (a) into/out of the whitelisted market, into the vault's own
/// accounts, or (b) to a user who burns shares, pro-rata.
#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub manager: Pubkey,
    pub market: Pubkey,
    pub base_mint: Pubkey,
    pub position_mint: Pubkey,
    pub share_mint: Pubkey,
    pub bump: u8,
    pub share_mint_bump: u8,
}

/// Total value of the vault in base units. The mock market's position
/// token is a 1:1 receipt for base tokens, so it is valued at par.
pub fn total_assets(idle: u64, position: u64) -> Result<u64> {
    idle.checked_add(position).ok_or(error!(VaultError::MathOverflow))
}

/// Shares minted for depositing `amount` base tokens (rounded down, in the
/// vault's favour). The first depositor gets shares 1:1.
pub fn shares_for_deposit(amount: u64, supply: u64, total_assets: u64) -> Result<u64> {
    if supply == 0 || total_assets == 0 {
        return Ok(amount);
    }
    mul_div(amount, supply, total_assets)
}

/// `balance * shares / supply`, rounded down (in the vault's favour).
pub fn pro_rata(balance: u64, shares: u64, supply: u64) -> Result<u64> {
    require!(supply > 0, VaultError::NothingToRedeem);
    mul_div(balance, shares, supply)
}

fn mul_div(a: u64, b: u64, c: u64) -> Result<u64> {
    let r = (a as u128)
        .checked_mul(b as u128)
        .ok_or(error!(VaultError::MathOverflow))?
        / (c as u128);
    u64::try_from(r).map_err(|_| error!(VaultError::MathOverflow))
}
