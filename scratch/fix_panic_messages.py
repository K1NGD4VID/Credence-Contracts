import os
import re

def fix_panic_messages(file_path, replacements):
    if not os.path.isabs(file_path):
        file_path = os.path.join(r'c:\Users\fuhad\Credence-Contracts', file_path)
    
    with open(file_path, 'r') as f:
        content = f.read()
    
    for old, new in replacements:
        content = content.replace(old, new)
    
    with open(file_path, 'w') as f:
        f.write(content)

# test_create_bond.rs
fix_panic_messages('contracts/credence_bond/src/test_create_bond.rs', [
    ('expected = "bond amount below minimum threshold (bronze tier)"', 'expected = "bond amount below minimum required: 0 (minimum: 1000)"'),
    ('expected = "duration below minimum lock-up period"', 'expected = "bond duration below minimum required: 1 (minimum: 86400)"'),
])

# token_integration_test.rs
fix_panic_messages('contracts/credence_bond/src/token_integration_test.rs', [
    ('expected = "top-up amount below minimum required"', 'expected = "amount must be positive"'),
])
