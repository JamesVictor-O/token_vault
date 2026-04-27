use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        transfer_checked, close_account,
        Mint, TokenAccount, TokenInterface,
        TransferChecked, CloseAccount,
    },
};
 
declare_id!("7Pmj9K3BmKNcsD9uAEvb9mqcckx9qybwy9dXH3TLWbD");

#[program]
pub mod token_vault {
   use super::*;
 
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }
 
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }
 
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }
 
    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()
    }

}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
 
    pub mint: InterfaceAccount<'info, Mint>,
 
    #[account(
        init,
        payer = user,
        seeds = [b"state", user.key().as_ref()],
        bump,
        space = VaultState::INIT_SPACE,
    )]
    pub vault_state: Account<'info, VaultState>,
 
    #[account(
        init,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = vault_state,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
 
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        self.vault_state.vault_bump = bumps.vault_state;
        self.vault_state.state_bump = bumps.vault_state;
        Ok(())
    }
}



#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
 
    pub mint: InterfaceAccount<'info, Mint>,
 
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,
 
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault_state,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
 
    #[account(
        seeds = [b"state", user.key().as_ref()],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
 
    pub token_program: Interface<'info, TokenInterface>,
}


impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
 
        // Step 1: Which program are we calling?
        let cpi_program = self.token_program.to_account_info();
 
        // Step 2: Which accounts does that program need?
        let cpi_accounts = TransferChecked {
            from: self.user_ata.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.user.to_account_info(),
            mint: self.mint.to_account_info(),
        };
 
        // Step 3: Build the CPI context
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
 
        // Step 4: Make the call!
        transfer_checked(cpi_ctx, amount, self.mint.decimals)?;
 
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
 
    pub mint: InterfaceAccount<'info, Mint>,
 
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,
 
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault_state,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
 
    #[account(
        seeds = [b"state", user.key().as_ref()],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
 
    pub token_program: Interface<'info, TokenInterface>,
}


impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
 
        // Step 1: Program to call
        let cpi_program = self.token_program.to_account_info();
 
        // Step 2: Accounts (notice from/to are swapped vs deposit)
        let cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),       // FROM the vault
            to: self.user_ata.to_account_info(),      // TO the user
            authority: self.vault_state.to_account_info(), // PDA authorizes
            mint: self.mint.to_account_info(),
        };
        // Step 3: Build the seeds for PDA signing
        let seeds = &[
            b"state",
            self.user.to_account_info().key.as_ref(),
            &[self.vault_state.state_bump],
        ];
        let signer_seeds = &[&seeds[..]];
 
        // Step 4: CPI with signer (not regular CPI!)
        let cpi_ctx = CpiContext::new_with_signer(
            cpi_program, cpi_accounts, signer_seeds,
        );
 
        // Step 5: Execute
        transfer_checked(cpi_ctx, amount, self.mint.decimals)?;
 
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
 
    pub mint: InterfaceAccount<'info, Mint>,
 
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault_state,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
 
    #[account(
        mut,
        seeds = [b"state", user.key().as_ref()],
        bump = vault_state.state_bump,
        close = user,
    )]
    pub vault_state: Account<'info, VaultState>,
 
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}


impl<'info> Close<'info> {
    pub fn close(&mut self) -> Result<()> {
 
        // What we're doing: closing the vault's Token Account
        // This sends the rent back to the user
        let cpi_accounts = CloseAccount {
            account: self.vault.to_account_info(),    // account to close
            destination: self.user.to_account_info(), // where rent goes
            authority: self.vault_state.to_account_info(), // who authorizes
        };
 
        // Same PDA signer pattern (vault_state is a PDA)
        let seeds = &[
            b"state",
            self.user.to_account_info().key.as_ref(),
            &[self.vault_state.state_bump],
        ];
        let signer_seeds = &[&seeds[..]];
 
        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts, signer_seeds,
        );
 
        close_account(cpi_ctx)?;
 
        Ok(())
    }
}






#[account]
pub struct VaultState {
    pub vault_bump: u8,
    pub state_bump: u8,
}
 
impl Space for VaultState {
    const INIT_SPACE: usize = 8 + 1 + 1;
}

