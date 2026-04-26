import re
import os

file_path = r'c:\Users\fuhad\Credence-Contracts\contracts\credence_bond\src\test_upgrade_auth.rs'

with open(file_path, 'r', encoding='utf-8') as f:
    lines = f.readlines()

output = []
in_test = False
current_test_body = []
test_header = ""

for line in lines:
    if line.strip() == "#[test]":
        in_test = True
        test_header = line
        current_test_body = []
        continue
    
    if in_test:
        current_test_body.append(line)
        if line.strip() == "}":
            # End of test
            # Process body
            body = "".join(current_test_body)
            # Find function name
            match = re.match(r'\s*fn\s+(\w+)\(\)\s+\{(.*)\}', body, re.DOTALL)
            if match:
                fn_name = match.group(1)
                fn_body = match.group(2)
                
                # Check if already wrapped
                if "as_contract" in fn_body:
                    output.append(test_header)
                    output.append(body)
                else:
                    # Wrap it
                    # Clean up env/admin setup
                    fn_body = re.sub(r'let env = create_test_env\(\);\s*', '', fn_body)
                    fn_body = re.sub(r'let admin = create_test_address\(&env\);\s*', '', fn_body)
                    
                    # Fix UpgradeRole -> upgrade_auth::UpgradeRole
                    fn_body = fn_body.replace("UpgradeRole::", "upgrade_auth::UpgradeRole::")
                    fn_body = fn_body.replace("UpgradeStatus::", "upgrade_auth::UpgradeStatus::")
                    
                    new_test = f"""{test_header}
fn {fn_name}() {{
    let env = create_test_env();
    let (contract_id, admin) = setup_upgrade_test(&env);
    env.mock_all_auths();
    env.as_contract(&contract_id, || {{
{fn_body}
    }});
}}
"""
                    output.append(new_test)
            else:
                # Might be partial match or something else
                output.append(test_header)
                output.append(body)
            in_test = False
    else:
        output.append(line)

with open(file_path, 'w', encoding='utf-8') as f:
    f.writelines(output)
