use arcis::prelude::*;

#[encrypted]
mod circuits {
    use arcis::prelude::*;

    pub struct PositionInput {
        pub size: u64,
        pub entry_price: u64,
        pub is_short: u8,
        pub leverage: u8,
    }

    pub struct PriceFeedInput {
        pub mark_price: u64,
        pub funding_rate_bps: u64,
        pub timestamp: u64,
    }

    pub struct OrderInput {
        pub size: u64,
        pub limit_price: u64,
        pub is_short: u8,
        pub leverage: u8,
    }

    pub struct PnlOutput {
        pub pnl_usd_cents: u64,
        pub is_profit: u8,
    }

    #[instruction]
    pub fn open_position(
        order: Enc<Shared, OrderInput>,
    ) -> Enc<Shared, PositionInput> {
        let o = order.to_arcis();
        let safe_leverage = if o.leverage < 1u8 {
            1u8
        } else if o.leverage > 100u8 {
            100u8
        } else {
            o.leverage
        };
        let position = PositionInput {
            size: o.size,
            entry_price: o.limit_price,
            is_short: o.is_short,
            leverage: safe_leverage,
        };
        order.owner.from_arcis(position)
    }

    #[instruction]
    pub fn liquidation_check(
        position: Enc<Shared, PositionInput>,
        price_feed: Enc<Shared, PriceFeedInput>,
    ) -> Enc<Shared, u8> {
        let pos = position.to_arcis();
        let feed = price_feed.to_arcis();
        let lev = pos.leverage as u64;
        let maint_bps = 10_000u64 / lev;
        let liq_dist_bps = (10_000u64 / lev).saturating_sub(maint_bps);
        let liq_price_long = pos.entry_price.saturating_sub(
            pos.entry_price * liq_dist_bps / 10_000u64
        );
        let liq_price_short = pos.entry_price + (pos.entry_price * liq_dist_bps / 10_000u64);
        let liq_long  = (pos.is_short == 0u8) as u8 * (feed.mark_price <= liq_price_long) as u8;
        let liq_short = (pos.is_short == 1u8) as u8 * (feed.mark_price >= liq_price_short) as u8;
        let should_liquidate = ((liq_long | liq_short) > 0u8) as u8;
        position.owner.from_arcis(should_liquidate)
    }

    #[instruction]
    pub fn close_position(
        position: Enc<Shared, PositionInput>,
        price_feed: Enc<Shared, PriceFeedInput>,
    ) -> Enc<Shared, PnlOutput> {
        let pos = position.to_arcis();
        let feed = price_feed.to_arcis();
        let entry_notional = pos.size / 1_000_000u64 * pos.entry_price;
        let exit_notional  = pos.size / 1_000_000u64 * feed.mark_price;
        let long_profit  = (pos.is_short == 0u8) as u8 & (exit_notional >= entry_notional) as u8;
        let short_profit = (pos.is_short == 1u8) as u8 & (entry_notional >= exit_notional) as u8;
        let is_profit    = ((long_profit | short_profit) > 0u8) as u8;
        let raw_pnl = if long_profit > 0u8 {
            exit_notional.saturating_sub(entry_notional)
        } else if short_profit > 0u8 {
            entry_notional.saturating_sub(exit_notional)
        } else if pos.is_short == 0u8 {
            entry_notional.saturating_sub(exit_notional)
        } else {
            exit_notional.saturating_sub(entry_notional)
        };
        let leveraged_pnl = raw_pnl * (pos.leverage as u64);
        let funding_cost = pos.size * feed.funding_rate_bps / 10_000u64 / 1_000_000u64;
        let final_pnl = leveraged_pnl.saturating_sub(funding_cost);
        let result = PnlOutput {
            pnl_usd_cents: final_pnl,
            is_profit,
        };
        position.owner.from_arcis(result)
    }
}
