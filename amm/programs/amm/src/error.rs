use anchor_lang::prelude::*;

#[error_code]
pub enum AMMError {
    #[msg("Token A and Token B must be different.")]
    SameTokenMint,
    #[msg("Wrong Token Mint")]
    InvalidTokenMint,
    #[msg("Token deposit amounts are not proportional to existing pool reserves.")]
    InvalidLiquidity,
    #[msg("Arithmetic operation overflow or division by zero.")]
    ArithmeticOverflow,
}
