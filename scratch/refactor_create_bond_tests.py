
import os
import re

file_path = r'c:\Users\fuhad\Credence-Contracts\contracts\credence_bond\src\test_create_bond.rs'

with open(file_path, 'r') as f:
    content = f.read()

# Replace manual setup with setup_with_token
# Pattern: let e = Env::default(); ... let client = ...; let admin = ...; client.initialize(&admin); let identity = ...;
# We want to replace it with let (client, admin, identity, token_id, bond_id) = test_helpers::setup_with_token(&e);

def replace_setup(match):
    e_name = match.group(1)
    return f'let {e_name} = Env::default();\n    let (client, admin, identity, _token_id, _bond_id) = test_helpers::setup_with_token(&{e_name});'

content = re.sub(r'let (\w+) = Env::default\(\);\s+let contract_id = \1\.register\(CredenceBond, \(\)\);\s+let client = CredenceBondClient::new\(&\1, &contract_id\);\s+(?:e\.mock_all_auths\(\);\s+)?let admin = Address::generate\(&\1\);\s+(?:e\.mock_all_auths\(\);\s+)?client\.initialize\(&admin\);\s+let identity = Address::generate\(&\1\);', replace_setup, content)

# Also fix tests that only had partial setup
content = re.sub(r'let (\w+) = Env::default\(\);\s+let contract_id = \1\.register\(CredenceBond, \(\)\);\s+let client = CredenceBondClient::new\(&\1, &contract_id\);\s+(?:e\.mock_all_auths\(\);\s+)?let admin = Address::generate\(&\1\);\s+(?:e\.mock_all_auths\(\);\s+)?client\.initialize\(&admin\);', 
                 r'let \1 = Env::default();\n    let (client, admin, _identity, _token_id, _bond_id) = test_helpers::setup_with_token(&\1);', content)

with open(file_path, 'w') as f:
    f.write(content)
