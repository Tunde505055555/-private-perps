use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

const COMP_DEF_OFFSET_OPEN_POSITION:  u32 = comp_def_offset("open_position");
const COMP_DEF_OFFSET_LIQ_CHECK:      u32 = comp_def_offset("liquidation_check");
const COMP_DEF_OFFSET_CLOSE_POSITION: u32 = comp_def_offset("close_position");

declare_id!("PPERPSxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");

#[arcium_program]
pub mod private_perps {
    use super::*;

    pub fn init_open_position_comp_def(
        ctx: Context<InitOpenPositionCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn init_liquidation_check_comp_def(
        ctx: Context<InitLiquidationCheckCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn init_close_position_comp_def(
        ctx: Context<InitClosePositionCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn open_position(
        ctx: Context<OpenPosition>,
        computation_offset: u64,
        enc_size:        [u8; 32],
        enc_limit_price: [u8; 32],
        enc_is_short:    [u8; 32],
        enc_leverage:    [u8; 32],
        pub_key: [u8; 32],
        nonce:   u128,
    ) -> Result<()> {
        let args = ArgBuilder::new()
            .x25519_pubkey(pub_key)
            .plaintext_u128(nonce)
            .encrypted_u64(enc_size)
            .encrypted_u64(enc_limit_price)
            .encrypted_u8(enc_is_short)
            .encrypted_u8(enc_leverage)
            .build();

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![OpenPositionCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;

        emit!(PositionOpenedEvent {
            trader: ctx.accounts.trader.key(),
            computation_offset,
        });

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "open_position")]
    pub fn open_position_callback(
        ctx: Context<OpenPositionCallback>,
        output: SignedComputationOutputs<OpenPositionOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(OpenPositionOutput { field_0 }) => field_0,
            Err(e) => {
                msg!("MPC error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let account = &mut ctx.accounts.position_account;
        account.trader         = ctx.accounts.trader.key();
        account.enc_position   = o.ciphertexts[0];
        account.position_nonce = o.nonce.to_le_bytes();
        account.is_open        = true;
        account.opened_at      = Clock::get()?.unix_timestamp;
        Ok(())
    }

    pub fn liquidation_check(
        ctx: Context<LiquidationCheck>,
        computation_offset: u64,
        enc_mark_price:   [u8; 32],
        enc_funding_rate: [u8; 32],
        enc_timestamp:    [u8; 32],
        pub_key: [u8; 32],
        nonce:   u128,
    ) -> Result<()> {
        require!(ctx.accounts.position_account.is_open, ErrorCode::PositionNotOpen);

        let args = ArgBuilder::new()
            .x25519_pubkey(pub_key)
            .plaintext_u128(nonce)
            .encrypted_bytes(ctx.accounts.position_account.enc_position)
            .encrypted_u64(enc_mark_price)
            .encrypted_u64(enc_funding_rate)
            .encrypted_u64(enc_timestamp)
            .build();

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![LiquidationCheckCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "liquidation_check")]
    pub fn liquidation_check_callback(
        ctx: Context<LiquidationCheckCallback>,
        output: SignedComputationOutputs<LiquidationCheckOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(LiquidationCheckOutput { field_0 }) => field_0,
            Err(e) => {
                msg!("MPC error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        emit!(LiquidationCheckEvent {
            trader:               ctx.accounts.position_account.trader,
            enc_should_liquidate: o.ciphertexts[0],
            nonce:                o.nonce.to_le_bytes(),
        });
        Ok(())
    }

    pub fn close_position(
        ctx: Context<ClosePosition>,
        computation_offset: u64,
        enc_mark_price:   [u8; 32],
        enc_funding_rate: [u8; 32],
        enc_timestamp:    [u8; 32],
        pub_key: [u8; 32],
        nonce:   u128,
    ) -> Result<()> {
        require!(ctx.accounts.position_account.is_open, ErrorCode::PositionNotOpen);
        require!(
            ctx.accounts.position_account.trader == ctx.accounts.trader.key(),
            ErrorCode::Unauthorized
        );

        let args = ArgBuilder::new()
            .x25519_pubkey(pub_key)
            .plaintext_u128(nonce)
            .encrypted_bytes(ctx.accounts.position_account.enc_position)
            .encrypted_u64(enc_mark_price)
            .encrypted_u64(enc_funding_rate)
            .encrypted_u64(enc_timestamp)
            .build();

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![ClosePositionCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "close_position")]
    pub fn close_position_callback(
        ctx: Context<ClosePositionCallback>,
        output: SignedComputationOutputs<ClosePositionOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(ClosePositionOutput { field_0 }) => field_0,
            Err(e) => {
                msg!("MPC error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let account = &mut ctx.accounts.position_account;
        account.is_open   = false;
        account.closed_at = Clock::get()?.unix_timestamp;
        emit!(PositionClosedEvent {
            trader:     account.trader,
            enc_pnl:    o.ciphertexts[0],
            enc_profit: o.ciphertexts[1],
            pnl_nonce:  o.nonce.to_le_bytes(),
            closed_at:  account.closed_at,
        });
        Ok(())
    }
}

#[account]
pub struct PositionAccount {
    pub trader:         Pubkey,
    pub enc_position:   [u8; 32],
    pub position_nonce: [u8; 16],
    pub is_open:        bool,
    pub opened_at:      i64,
    pub closed_at:      i64,
}

impl PositionAccount {
    pub const LEN: usize = 8 + 32 + 32 + 16 + 1 + 8 + 8;
}

#[event]
pub struct PositionOpenedEvent {
    pub trader:             Pubkey,
    pub computation_offset: u64,
}

#[event]
pub struct LiquidationCheckEvent {
    pub trader:               Pubkey,
    pub enc_should_liquidate: [u8; 32],
    pub nonce:                [u8; 16],
}

#[event]
pub struct PositionClosedEvent {
    pub trader:    Pubkey,
    pub enc_pnl:   [u8; 32],
    pub enc_profit:[u8; 32],
    pub pnl_nonce: [u8; 16],
    pub closed_at: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Position is not open")]
    PositionNotOpen,
    #[msg("Only the position owner can close it")]
    Unauthorized,
    #[msg("MPC computation was aborted")]
    AbortedComputation,
}

#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    #[account(
        init, payer = trader,
        space = PositionAccount::LEN,
        seeds = [b"position", trader.key().as_ref()], bump
    )]
    pub position_account: Account<'info, PositionAccount>,
    #[account(mut)] pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut)] pub mempool_account: Account<'info, MempoolAccount>,
    #[account(mut)] pub comp_def_account: Account<'info, CompDefAccount>,
    #[account(init, payer = trader, space = ComputationAccount::LEN,
        seeds = [b"computation", &computation_offset.to_le_bytes()], bump,
        seeds::program = arcium_program::ID)]
    pub computation_account: Account<'info, ComputationAccount>,
    #[account(mut)] pub cluster_account: Account<'info, ClusterAccount>,
    #[account(mut)] pub executing_pool: Account<'info, ExecutingPool>,
    #[account(init_if_needed, payer = trader, space = SignPDAAccount::LEN,
        seeds = [b"sign_pda", program_id().as_ref()], bump)]
    pub sign_pda_account: Account<'info, SignPDAAccount>,
    pub arcium_program: Program<'info, arcium_program::program::Arcium>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct OpenPositionCallback<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    /// CHECK: validated by Arcium
    pub trader: AccountInfo<'info>,
    #[account(mut, seeds = [b"position", trader.key().as_ref()], bump)]
    pub position_account: Account<'info, PositionAccount>,
    #[account(mut)] pub cluster_account: Account<'info, ClusterAccount>,
    #[account(mut, seeds = [b"computation", &computation_offset.to_le_bytes()],
        bump, seeds::program = arcium_program::ID)]
    pub computation_account: Account<'info, ComputationAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct LiquidationCheck<'info> {
    #[account(mut)] pub keeper: Signer<'info>,
    /// CHECK: validated by seeds
    pub trader: AccountInfo<'info>,
    #[account(mut, seeds = [b"position", trader.key().as_ref()], bump)]
    pub position_account: Account<'info, PositionAccount>,
    #[account(mut)] pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut)] pub mempool_account: Account<'info, MempoolAccount>,
    #[account(mut)] pub comp_def_account: Account<'info, CompDefAccount>,
    #[account(init, payer = keeper, space = ComputationAccount::LEN,
        seeds = [b"computation", &computation_offset.to_le_bytes()], bump,
        seeds::program = arcium_program::ID)]
    pub computation_account: Account<'info, ComputationAccount>,
    #[account(mut)] pub cluster_account: Account<'info, ClusterAccount>,
    #[account(mut)] pub executing_pool: Account<'info, ExecutingPool>,
    #[account(init_if_needed, payer = keeper, space = SignPDAAccount::LEN,
        seeds = [b"sign_pda", program_id().as_ref()], bump)]
    pub sign_pda_account: Account<'info, SignPDAAccount>,
    pub arcium_program: Program<'info, arcium_program::program::Arcium>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct LiquidationCheckCallback<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    /// CHECK: read-only
    pub trader: AccountInfo<'info>,
    #[account(seeds = [b"position", trader.key().as_ref()], bump)]
    pub position_account: Account<'info, PositionAccount>,
    #[account(mut)] pub cluster_account: Account<'info, ClusterAccount>,
    #[account(mut, seeds = [b"computation", &computation_offset.to_le_bytes()],
        bump, seeds::program = arcium_program::ID)]
    pub computation_account: Account<'info, ComputationAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct ClosePosition<'info> {
    #[account(mut)] pub trader: Signer<'info>,
    #[account(mut, seeds = [b"position", trader.key().as_ref()], bump)]
    pub position_account: Account<'info, PositionAccount>,
    #[account(mut)] pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut)] pub mempool_account: Account<'info, MempoolAccount>,
    #[account(mut)] pub comp_def_account: Account<'info, CompDefAccount>,
    #[account(init, payer = trader, space = ComputationAccount::LEN,
        seeds = [b"computation", &computation_offset.to_le_bytes()], bump,
        seeds::program = arcium_program::ID)]
    pub computation_account: Account<'info, ComputationAccount>,
    #[account(mut)] pub cluster_account: Account<'info, ClusterAccount>,
    #[account(mut)] pub executing_pool: Account<'info, ExecutingPool>,
    #[account(init_if_needed, payer = trader, space = SignPDAAccount::LEN,
        seeds = [b"sign_pda", program_id().as_ref()], bump)]
    pub sign_pda_account: Account<'info, SignPDAAccount>,
    pub arcium_program: Program<'info, arcium_program::program::Arcium>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct ClosePositionCallback<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut)] pub trader: Signer<'info>,
    #[account(mut, seeds = [b"position", trader.key().as_ref()], bump)]
    pub position_account: Account<'info, PositionAccount>,
    #[account(mut)] pub cluster_account: Account<'info, ClusterAccount>,
    #[account(mut, seeds = [b"computation", &computation_offset.to_le_bytes()],
        bump, seeds::program = arcium_program::ID)]
    pub computation_account: Account<'info, ComputationAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitOpenPositionCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut)] pub mxe_account: Account<'info, MXEAccount>,
    #[account(init, payer = payer, space = CompDefAccount::LEN,
        seeds = [b"comp_def", program_id().as_ref(),
        &COMP_DEF_OFFSET_OPEN_POSITION.to_le_bytes()], bump,
        seeds::program = arcium_program::ID)]
    pub comp_def_account: Account<'info, CompDefAccount>,
    pub arcium_program: Program<'info, arcium_program::program::Arcium>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitLiquidationCheckCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut)] pub mxe_account: Account<'info, MXEAccount>,
    #[account(init, payer = payer, space = CompDefAccount::LEN,
        seeds = [b"comp_def", program_id().as_ref(),
        &COMP_DEF_OFFSET_LIQ_CHECK.to_le_bytes()], bump,
        seeds::program = arcium_program::ID)]
    pub comp_def_account: Account<'info, CompDefAccount>,
    pub arcium_program: Program<'info, arcium_program::program::Arcium>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitClosePositionCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut)] pub mxe_account: Account<'info, MXEAccount>,
    #[account(init, payer = payer, space = CompDefAccount::LEN,
        seeds = [b"comp_def", program_id().as_ref(),
        &COMP_DEF_OFFSET_CLOSE_POSITION.to_le_bytes()], bump,
        seeds::program = arcium_program::ID)]
    pub comp_def_account: Account<'info, CompDefAccount>,
    pub arcium_program: Program<'info, arcium_program::program::Arcium>,
    pub system_program: Program<'info, System>,
}
