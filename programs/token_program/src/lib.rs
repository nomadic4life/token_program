use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};

declare_id!("82X9jUhf5wT8n3RvnDDhh7wYtPJPwTqFLUWTgaGLWkts");

#[program]
pub mod token_program {
    use super::*;

    // initialize program signer account
    //  expected accounts:
    //      -> {payer_account, isSigner, isWritable}
    //      -> {program.signer_account, !isSigner, isWritable}
    //      -> {system_account, !isSigner, !isWritable}
    //  state accounts:
    //      -> [a] program.signer_account
    //  state accounts derived:
    //      -> [a] ["signer", bump], program_id
    //  expected accounts init:
    //      -> program.signer_account
    //  program signer:
    //  validation:
    //  process:
    //      -> ?? check         : check if account doesn't exist, prevent possible overide of data
    //      -> create account   : INVOKE    -> system_program::create_account
    //      ->                  : INPUT     -> payer.pubkey             :from_pubkey:
    //      ->                  :           -> program.signer.pubkey    :to_pubkey:
    //      ->                  :           -> lamports
    //      ->                  :           -> space
    //      ->                  :           -> program_id               :owner:
    //      -> INIT STATE       : INIT      -> isInitialized = true, isSigner = true, bump
    pub fn initialize_program_signer(
        ctx: Context<InitializeProgramSigner>,
        bump: u8,
    ) -> Result<()> {
        ctx.accounts.new_program_signer.is_initialized = true;
        ctx.accounts.new_program_signer.is_signer = true;
        ctx.accounts.new_program_signer.bump = bump;

        msg!(
            "program signer is initialized new pubkey: {}",
            ctx.accounts.new_program_signer.key()
        );

        return Ok(());
    }

    // initialize associated token accounts
    //  expected accounts:
    //      -> {payer_account, isSigner, isWritable}
    //      -> {program.signer_account, isSigner, !isWritable}
    //      -> {program.signer.associated_token_account, !isSigner, isWritable}
    //      -> {token_mint_account, !isSigner, !isWritable}
    //      -> {token_program_account, !isSigner, !isWritable}
    //      -> {system_program_account, !isSigner, !isWritable}
    //  state accounts:
    //      -> [a] program.signer.associated_token_account
    //  state accounts derived:
    //      -> [a] [program.signer_account, token_program_account, token_mint_account], associated_token_program_id
    //  expected accounts init:
    //      -> program.signer.associated_token_account      intentional_use[signer, state]
    //  program signer:
    //  validation:
    //  process:
    //      -> ?? check         :           -> check if program.signer.associated_token_account doesn't exist
    //      -> create account   : INVOKE    -> associated_token_program::create_associated_token_account
    //      ->                  : INPUT     -> payer.pubkey            :funding_address:
    //      ->                  :           -> program.signer.pubkey   :wallet_address:
    //      ->                  :           -> token_mint.pubkey
    //      ->                  :           -> token_program_id
    pub fn initialize_program_associate_token_account(
        ctx: Context<InitializeProgramAssociatedTokenAccount>,
    ) -> Result<()> {
        msg!(
            "program associated token account intialzied new pubkey: {}",
            ctx.accounts.program_associated_token_account.key()
        );

        return Ok(());
    }

    // initialize locked token accounts
    //  expected accounts:
    //      -> {user_account, isSigner, isWritable}
    //      -> {program.locked_account, !isSigner, isWritable}
    //      -> {user.associated_token_account, !isSigner, !isWritable}
    //      -> {program.signer.associated_token_account, !isSigner, !isWritable}
    //      -> {system_program_account, !isSigner, !isWritable}
    //  state Accounts:
    //      -> [a] program.locked_account
    //      -> [b] user.associated_token_account
    //      -> [c] program.signer.associated_token_account
    //  state accounts derived:
    //      -> [a] [user.pubkey, program.signer.pubkey, token_mint.pubkey, "locked"], program_id
    //  expected accounts init:
    //      -> program.locked_account       : intentional_use [state]
    //  program signer:
    //  validation:
    //      ->  user.associated_token_account.mint == program.associated_token_account.mint
    //      ->  program.locked_account.pubkey == derived.pubkey
    //  process: (input: token_mint.pubkey)
    //      -> create account   : INVOKE    -> system_program::create_account
    //      ->                  : INPUT     -> user.pubkey             :from_pubkey:
    //      ->                  :           -> program.locked.pubkey   :to_pubkey:
    //      ->                  :           -> lamports
    //      ->                  :           -> space
    //      ->                  :           -> program_id               :owner:
    //      -> init state       : INIT      ->
    pub fn initialize_locked_token_account(
        ctx: Context<InitializeLockedTokenAccount>,
    ) -> Result<()> {
        let a = ctx.accounts;

        a.program_locked_account.authority = a.program_signer.key();
        a.program_locked_account.token_mint = a.token_mint.key();
        a.program_locked_account.amount = 0;

        msg!(
            "program locked account intialzied new pubkey: {}",
            a.program_locked_account.key()
        );

        msg!("locked amount: {}", a.program_locked_account.amount);

        return Ok(());
    }

    // stake token
    //  expected accounts:
    //      -> {user_account, isSigner, ?isWritable}                                : fee_payer
    //      -> {user.associcated_token_account, !isSigner, isWritable}
    //      -> {program.signer.associated_token_account, !isSigner, isWritable}
    //      -> {program.user+token.locked_account, !isSigner, isWritable}
    //      -> ?{associated_token_program, !isSigner, !isWritable}
    //      -> {token_program, !isSigner, !isWritable}
    //      -> ?{system_program, !isSigner, !isWritable}
    //  state accounts:
    //      -> [a] user.associated_token_account            -> transfer token from
    //      -> [b] program.signer.assoicated_token_account  -> transfer token to        -> token staking pool
    //      -> [c] program.user+token.locked_account        -> user token amount that is staked
    //  state accounts derived:
    //      -> [a] [user.pubkey, token_program_id, token_mint.pubkey], associated_token_program_id              : canonincal bump
    //      -> [a] [program.signer.pubkey, token_program_id, token_mint.pubkey], associated_token_program_id    : canonincal bump
    //      -> [c] [user.pubkey, token_mint.pubkey, program.signer.pubkey, "locked"], program_id                : canonincal bump
    //  expected accounts init:
    //      -> ?program.signer.associated_token_account     : intentional_use [state]
    //      -> ?program.user+token.locked_account           : intentional_use [state]
    //  program signer:
    //      -> !program.signer_account
    //  program signer derived:
    //      -> ["signer"], program_id       : canonical bump
    //  validation:
    //  process: (input: amount)
    //      -> transfer : INVOKE    -> token_program::transfer
    //      ->          : INPUT     -> user.associated_token_account                :from:
    //      ->          :           -> program.signer.associated_token_account      :to:
    //      ->          :           -> user_account                                 :authority:
    //      -> update   : --        -> program.user+token.locked_account.amount  =  amount
    //      ->          :           -> program.user+token.lcoked_account.authority  =  user.pubkey
    pub fn stake_token(ctx: Context<StakeToken>, amount: u64) -> Result<()> {
        let a = ctx.accounts;

        let cpi_program = a.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: a.user_associated_token.to_account_info(),
            to: a.program_associated_token.to_account_info(),
            authority: a.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(cpi_ctx, amount)?;

        a.locked_token.amount += amount;

        msg!("locked amount: {}", a.locked_token.amount);

        return Ok(());
    }

    // unstake token
    //  expected accounts:
    //      -> {user_account, isSigner, ?isWritable} -> fee_payer
    //      -> {program.signer.associated_token_account, !isSigner, isWritable}
    //      -> {user.associcated_token_account, !isSigner, isWritable}
    //      -> {program.signer_account, !isSigner, isWritable}
    //      -> {token_program_account, !isSigner, !isWritable}
    //  state accounts:
    //      -> [a] program.signer.associated_token_account      : transfer token from
    //      -> [b] user.associated_token_account                : transfer token to
    //      -> [c] program.user+token.locked_account            : user token amount that is unstaked
    //  state accounts derived:
    //      -> [a] [program.signer.pubkey, token_program_id, token_mint.pubkey], associated_token_program_id
    //      -> [b] [user.pubkey, token_program_id, token_mint.pubkey], associated_token_program_id
    //      -> [c] [user.pubkey, token_mint.pubkey, program.signer.pubkey, "locked"], program_id
    //  expected accounts init:
    //  program signer:
    //      -> program.signer_account       : validate, unlock, and transfer tokens from token staking pool to user token account
    //  program signer derived:
    //      -> ["signer"], program_id       : canonical bump
    //  validation:
    //  process: (input: amount)        : the amount to unstake, can't unstake more than what is staked.
    //      -> verity       : --        -> program.user+token.locked_account.authority  ==  user.pubkey
    //      -> check        : --        -> amount  <=  program.user+token.locked_account.amount
    //      -> transfer     : INVOKE    -> token_program::transfer
    //      ->              : INPUT     -> program.signer.associated_token_account      :from:
    //      ->              :           -> user.associated_token_account                :to:
    //      ->              :           -> program.signer_account                       :authority:
    //      -> update       : --        -> program.user+token.locked_account.amount  -=  amount
    pub fn unstake_token(ctx: Context<UnstakeToken>, amount: u64) -> Result<()> {
        let a = ctx.accounts;

        let bump = a.program_signer.bump.to_le_bytes();
        let inner=vec!["signer".as_ref(),bump.as_ref()];
        let outer=vec![inner.as_slice()];
        let cpi_program = a.associated_token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: a.program_associated_token.to_account_info(),
            to: a.user_associated_token.to_account_info(),
            authority: a.program_signer.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, &outer.as_ref());

        transfer(cpi_ctx, amount)?;

        a.locked_token.amount -= amount;

        msg!("locked amount: {}", a.locked_token.amount);

        return Ok(());
    }
}
#[error_code]
pub enum MyError {
    #[msg("User can't unstake amount more than locked balance.")]
    AmountTooLarge
}

#[derive(Accounts)]
pub struct InitializeProgramSigner<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + 1 + 1 + 1,
        seeds = [b"signer"],
        bump
    )]
    pub new_program_signer: Account<'info, SignerAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeProgramAssociatedTokenAccount<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        constraint = program_signer.is_initialized == true,
        seeds = [b"signer"],
        bump = program_signer.bump
    )]
    pub program_signer: Account<'info, SignerAccount>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = token_mint,
        associated_token::authority = program_signer,
        associated_token::token_program = token_program,
        // associated_token::token_program = associated_token_program,

    )]
    pub program_associated_token_account: Account<'info, TokenAccount>,
    pub token_mint: Account<'info, Mint>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeLockedTokenAccount<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"signer"],
        bump = program_signer.bump
    )]
    pub program_signer: Account<'info, SignerAccount>,

    #[account(
        init,
        payer = user,
        space = 8 + 32 + 32 + 8,
        seeds = [user.key().as_ref(), program_signer.key().as_ref(), token_mint.key().as_ref()],
        bump,
    )]
    pub program_locked_account: Account<'info, LockedTokenAccount>,

    #[account(
        constraint = user_associated_token.mint == program_associated_token.mint,
        seeds = [user.key().as_ref(), token_program.key().as_ref(), token_mint.key().as_ref()],
        bump,
        seeds::program = associated_token_program,
    )]
    pub user_associated_token: Account<'info, TokenAccount>,

    #[account(
        seeds = [
            program_signer.key().as_ref(), 
            token_program.key().as_ref(), 
            token_mint.key().as_ref()
        ],
        bump,
        seeds::program = associated_token_program
    )]
    pub program_associated_token: Account<'info, TokenAccount>,

    #[account(
        constraint = token_mint.key() == user_associated_token.mint && token_mint.key() == program_associated_token.mint
    )]
    pub token_mint: Account<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StakeToken<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"signer"],
        bump = program_signer.bump
    )]
    pub program_signer: Account<'info, SignerAccount>,

    #[account(
        mut,
        constraint = user_associated_token.mint == program_associated_token.mint,
        seeds = [user.key().as_ref(), token_program.key().as_ref(), token_mint.key().as_ref()],
        bump,
        seeds::program = associated_token_program,
    )]
    pub user_associated_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [program_signer.key().as_ref(), token_program.key().as_ref(), token_mint.key().as_ref()],
        bump,
        seeds::program = associated_token_program,
    )]
    pub program_associated_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [user.key().as_ref(), program_signer.key().as_ref(), token_mint.key().as_ref()],
        bump,
    )]
    pub locked_token: Account<'info, LockedTokenAccount>,

    #[account(
        constraint = token_mint.key() == user_associated_token.mint && token_mint.key() == program_associated_token.mint
    )]
    pub token_mint: Account<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct UnstakeToken<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"signer"],
        bump = program_signer.bump
    )]
    pub program_signer: Account<'info, SignerAccount>,

    #[account(
        mut,
        constraint = user_associated_token.mint == program_associated_token.mint,
        seeds = [user.key().as_ref(), token_program.key().as_ref(), token_mint.key().as_ref()],
        bump,
        seeds::program = associated_token_program,
    )]
    pub user_associated_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [program_signer.key().as_ref(), token_program.key().as_ref(), token_mint.key().as_ref()],
        bump,
        seeds::program = associated_token_program,
    )]
    pub program_associated_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = amount <= locked_token.amount @ MyError::AmountTooLarge,
        seeds = [user.key().as_ref(), program_signer.key().as_ref(), token_mint.key().as_ref()],
        bump,
    )]
    pub locked_token: Account<'info, LockedTokenAccount>,

    #[account(
        constraint = token_mint.key() == user_associated_token.mint && token_mint.key() == program_associated_token.mint
    )]
    pub token_mint: Account<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct SignerAccount {
    pub is_initialized: bool,
    pub is_signer: bool,
    pub bump: u8,
}

#[account]
pub struct LockedTokenAccount {
    // pubkey of user
    pub authority: Pubkey,

    // token mint
    pub token_mint: Pubkey,

    // staked amount of user
    pub amount: u64,
}

// For this assessment, you will build a Solana program to allow users to airdrop themselves tokens and then stake/unstake them. Below are the suggested steps and hints to build it.

// Create a Token Account that has a PDA derived from the program as the authority. Hint: the staking pool token account should probably also be a PDA

// When a user “stakes” their tokens, the program should transfer their tokens from the User’s token account and into the program’s token account.

// If a user wants to “unstake” their tokens, the PDA that is assigned the authority over the program’s token account should authorize the transfer of tokens back into the user’s wallet. Hint: create a CPI to the token program and use the staking pool authority to “sign” the transaction by passing in the seeds for the authority.

// Each user needs an additional account that keeps track of how many tokens they have staked. Hint: this could be a PDA using the user’s pubkey and a string (like “state_account”) as seeds.

// They should not be able to “unstake” more than they have staked. Hint: this would be a major security flaw if a user were able to withdraw more tokens than they have staked in a staking pool, allowing them to drain the entire pool if they wanted.
