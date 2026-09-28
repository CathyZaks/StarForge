# Wallet Passphrase Strength and Security

## Overview
StarForge uses local passphrase strength scoring to protect wallet encryption keys. All strength evaluation happens locally using the zxcvbn algorithm—no passphrases are ever sent to remote services.

## Passphrase Requirements

### Minimum Length
All passphrases must be at least **12 characters** long, regardless of complexity.

### Strength Scoring
Passphrases are scored on a 0-4 scale using the zxcvbn algorithm:

| Score | Label | Description |
|-------|-------|-------------|
| 0 | Very Weak | Trivially guessable (e.g., "password", "123456") |
| 1 | Weak | Easily guessable with minimal effort |
| 2 | Fair | Somewhat guessable, offers basic protection |
| 3 | Strong | Safely unguessable with current techniques |
| 4 | Very Strong | Extremely difficult to guess |

## Interactive Wallet Creation

When creating an encrypted wallet, StarForge provides real-time feedback:

```bash
$ starforge wallet create alice --encrypt
Creating wallet 'alice'

Step 1/2: Generating keypairâ€¦

Public Key: GABCDEFGHIJKLMNOPQRSTUVWXYZ123456789

Passphrase must be at least 12 characters. Add --strict to enforce a stronger passphrase.

Set a passphrase to encrypt this wallet: ****
  Strength: â–ˆâ–ˆâ–ˆâ–ˆâ–ˆ Very Strong
  💡 This is a strong passphrase.
Confirm passphrase: ****
Secret Key: Encrypted and safely stored.
```

### Interactive Feedback

The prompt shows:
- **Visual strength bar**: 5-segment bar with color coding (red/yellow/green)
- **Strength label**: Text description of the score
- **Warnings**: When passphrases reuse common patterns
- **Suggestions**: Tips to improve weak passphrases

## Strict Mode

Use `--strict` to enforce minimum security standards:

```bash
starforge wallet create alice --encrypt --strict
```

### Strict Mode Rules

1. **Minimum score**: Must achieve "Strong" (score 3) or better
2. **No context reuse**: Passphrase cannot contain wallet name, public key, or network name
3. **Enforcement**: Passphrases below the threshold are rejected

Example of strict mode rejection:

```bash
$ starforge wallet create alice --encrypt --strict
Set a passphrase to encrypt this wallet: alice_password_123
  Strength: â–ˆâ–ˆâ–ˆâ–ˆâ–ˆ Weak
  ⚠ Warning: this passphrase reuses wallet or account details.
  âœ— --strict mode requires a Strong or better passphrase. Please choose a stronger one.
```

## Non-Interactive Mode

For automated workflows (CI/CD, scripts), supply passphrases via environment variables:

```bash
# New passphrase for wallet creation
export STARFORGE_PASSPHRASE="your-strong-passphrase-here"
starforge wallet create deployer --encrypt

# Existing passphrase for decryption
export STARFORGE_PASSWORD="your-existing-passphrase"
starforge wallet show deployer --reveal
```

### Environment Variable Validation

Even in non-interactive mode, all validation rules apply:
- Minimum length check
- Strict mode enforcement (if `--strict` is used)
- Context reuse detection

If validation fails, the command exits with an error immediately (no retry loop).

## Security Properties

### Local-Only Scoring
- All passphrase strength evaluation uses the embedded zxcvbn library
- No network requests are made during strength checking
- Your passphrase never leaves your machine

### No Breach Database Checks
StarForge does not include breach database lookups by default. Users concerned about compromised passphrases should:

1. Use a password manager to generate unique passphrases
2. Follow the "Strong" (score 3+) guideline
3. Consider offline breach checking tools separately if needed

### Context-Aware Scoring
The strength scorer considers wallet-specific context:
- Wallet name
- Public key
- Network name

Passphrases containing these values score lower, as they're predictable to attackers who know your configuration.

## Best Practices

### Recommended Passphrases

âœ… **Good**: Long, random, or memorable phrases
- `correct-horse-battery-staple-789`
- `Tr0ub4dor&3-orchard-sunset-42`
- `OrchidRiverCopperHarbor2025!`

❌ **Avoid**: Dictionary words, common patterns, personal info
- `passwordpassword` (repeated word)
- `alice2024stellar` (contains wallet name)
- `qwerty123456789` (keyboard pattern)

### Production Deployments

For mainnet or production wallets:

1. **Always use `--encrypt`**: Never store plaintext secret keys
2. **Enable `--strict` mode**: Enforce strong passphrases
3. **Use a password manager**: Generate and store passphrases securely
4. **Document recovery**: Store encrypted backups and passphrase separately
5. **Rotate regularly**: Use `starforge wallet rotate` for long-lived wallets

### Hardware Wallet Integration

For maximum security, consider using hardware wallet signing (see Issue #731) instead of encrypted software wallets for production deployments.

## Configuration

### Global KDF Parameters

Adjust Argon2id parameters globally:

```bash
starforge config set-encryption --mem 65536 --iterations 4 --parallelism 2
```

Higher values increase computational cost for attackers but slow down encryption/decryption.

### Per-Wallet Override

Override KDF parameters for individual wallets:

```bash
starforge wallet create alice --encrypt --mem 131072 --iterations 5
```

## Testing

To verify passphrase strength without creating a wallet, examine the source tests:

```bash
cargo test --package starforge --lib utils::crypto::tests
```

Key test cases:
- `rejects_passphrase_shorter_than_minimum`
- `very_weak_passphrase_scores_low`
- `strong_passphrase_scores_high`
- `detects_passphrase_reusing_wallet_context`
- `prompt_passphrase_env_fallback_still_enforces_strict_strength`

## References

- [zxcvbn algorithm](https://github.com/dropbox/zxcvbn): Realistic password strength estimation
- [Argon2id](https://en.wikipedia.org/wiki/Argon2): Modern key derivation function
- [AES-256-GCM](https://en.wikipedia.org/wiki/Galois/Counter_Mode): Authenticated encryption
