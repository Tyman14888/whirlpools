use core::ops::Shr;

use ethnum::U256;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::{adjust_amount, CollectFeesQuote, Position, Tick, TransferFee, Whirlpool};

/// Calculate fees owed for a position
///
/// # Paramters
/// - `whirlpool`: The whirlpool state
/// - `position`: The position state
/// - `tick_lower`: The lower tick state
/// - `tick_upper`: The upper tick state
/// - `transfer_fee_a`: The transfer fee for token A
/// - `transfer_fee_b`: The transfer fee for token B
///
/// # Returns
/// - `CollectFeesQuote`: The fees owed for token A and token B
#[allow(clippy::too_many_arguments)]
#[cfg_attr(feature = "wasm", wasm_bindgen(js_name = collectFeesQuote, skip_jsdoc))]
pub fn collect_fees_quote(
    whirlpool: Whirlpool,
    position: Position,
    tick_lower: Tick,
    tick_upper: Tick,
    transfer_fee_a: Option<TransferFee>,
    transfer_fee_b: Option<TransferFee>,
) -> CollectFeesQuote {
    let mut fee_growth_below_a: u128 = tick_lower.fee_growth_outside_a;
    let mut fee_growth_above_a: u128 = tick_upper.fee_growth_outside_a;
    let mut fee_growth_below_b: u128 = tick_lower.fee_growth_outside_b;
    let mut fee_growth_above_b: u128 = tick_upper.fee_growth_outside_b;

    if whirlpool.tick_current_index < position.tick_lower_index {
        fee_growth_below_a = whirlpool
            .fee_growth_global_a
            .saturating_sub(fee_growth_below_a);
        fee_growth_below_b = whirlpool
            .fee_growth_global_b
            .saturating_sub(fee_growth_below_b);
    }

    if whirlpool.tick_current_index >= position.tick_upper_index {
        fee_growth_above_a = whirlpool
            .fee_growth_global_a
            .saturating_sub(fee_growth_above_a);
        fee_growth_above_b = whirlpool
            .fee_growth_global_b
            .saturating_sub(fee_growth_above_b);
    }

    let fee_growth_inside_a = whirlpool
        .fee_growth_global_a
        .saturating_sub(fee_growth_below_a)
        .saturating_sub(fee_growth_above_a);

    let fee_growth_inside_b = whirlpool
        .fee_growth_global_b
        .saturating_sub(fee_growth_below_b)
        .saturating_sub(fee_growth_above_b);

    let fee_owed_delta_a: U256 = <U256>::from(fee_growth_inside_a)
        .saturating_sub(position.fee_growth_checkpoint_a.into())
        .saturating_mul(position.liquidity.into())
        .shr(64);

    let fee_owed_delta_b: U256 = <U256>::from(fee_growth_inside_b)
        .saturating_sub(position.fee_growth_checkpoint_b.into())
        .saturating_mul(position.liquidity.into())
        .shr(64);

    let fee_owed_delta_a: u128 = fee_owed_delta_a.try_into().unwrap();
    let fee_owed_delta_b: u128 = fee_owed_delta_b.try_into().unwrap();

    let withdrawable_fee_a: u128 = position.fee_owed_a as u128 + fee_owed_delta_a;
    let withdrawable_fee_b: u128 = position.fee_owed_b as u128 + fee_owed_delta_b;

    let fee_owed_a = adjust_amount(withdrawable_fee_a.into(), transfer_fee_a.into(), false);
    let fee_owed_b = adjust_amount(withdrawable_fee_b.into(), transfer_fee_b.into(), false);

    CollectFeesQuote {
        fee_owed_a: fee_owed_a.into(),
        fee_owed_b: fee_owed_b.into(),
    }
}

#[cfg(all(test, not(feature = "wasm")))]
mod tests {
    use super::*;

    fn test_whirlpool(tick_index: i32) -> Whirlpool {
        Whirlpool {
            tick_current_index: tick_index,
            fee_growth_global_a: 800,
            fee_growth_global_b: 1000,
            ..Whirlpool::default()
        }
    }

    fn test_position() -> Position {
        Position {
            liquidity: 10000000000000000000,
            tick_lower_index: 5,
            tick_upper_index: 10,
            fee_growth_checkpoint_a: 300,
            fee_owed_a: 400,
            fee_growth_checkpoint_b: 500,
            fee_owed_b: 600,
            ..Position::default()
        }
    }

    fn test_tick() -> Tick {
        Tick {
            fee_growth_outside_a: 50,
            fee_growth_outside_b: 20,
            ..Tick::default()
        }
    }

    #[test]
    fn test_collect_out_of_range_lower() {
        let result = collect_fees_quote(
            test_whirlpool(0),
            test_position(),
            test_tick(),
            test_tick(),
            None,
            None,
        );
        assert_eq!(result.fee_owed_a, 400);
        assert_eq!(result.fee_owed_b, 600);
    }

    #[test]
    fn test_in_range() {
        let result = collect_fees_quote(
            test_whirlpool(7),
            test_position(),
            test_tick(),
            test_tick(),
            None,
            None,
        );
        assert_eq!(result.fee_owed_a, 616);
        assert_eq!(result.fee_owed_b, 849);
    }

    #[test]
    fn test_collect_out_of_range_upper() {
        let result = collect_fees_quote(
            test_whirlpool(15),
            test_position(),
            test_tick(),
            test_tick(),
            None,
            None,
        );
        assert_eq!(result.fee_owed_a, 400);
        assert_eq!(result.fee_owed_b, 600);
    }

    #[test]
    fn test_collect_on_range_lower() {
        let result = collect_fees_quote(
            test_whirlpool(5),
            test_position(),
            test_tick(),
            test_tick(),
            None,
            None,
        );
        assert_eq!(result.fee_owed_a, 616);
        assert_eq!(result.fee_owed_b, 849);
    }

    #[test]
    fn test_collect_on_upper() {
        let result = collect_fees_quote(
            test_whirlpool(10),
            test_position(),
            test_tick(),
            test_tick(),
            None,
            None,
        );
        assert_eq!(result.fee_owed_a, 400);
        assert_eq!(result.fee_owed_b, 600);
    }

    #[test]
    fn test_collect_transfer_fee() {
        let result = collect_fees_quote(
            test_whirlpool(7),
            test_position(),
            test_tick(),
            test_tick(),
            Some(TransferFee::new(2000)),
            Some(TransferFee::new(5000)),
        );
        assert_eq!(result.fee_owed_a, 492);
        assert_eq!(result.fee_owed_b, 424);
    }
}
