
import os
import re

file_path = r'c:\Users\fuhad\Credence-Contracts\contracts\credence_bond\src\test_create_bond.rs'

with open(file_path, 'r') as f:
    content = f.read()

# Add e.mock_all_auths() after client creation or before initialize
new_content = re.sub(r'(let client = CredenceBondClient::new\(&e, &contract_id\);)', r'\1\n    e.mock_all_auths();', content)

with open(file_path, 'w') as f:
    f.write(new_content)
