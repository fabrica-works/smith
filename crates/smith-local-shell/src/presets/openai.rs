//! The public endpoint and model catalog (shell.md, 6.1–6.2).
pub(super) const SETTINGS: &str = r"
[endpoints.openai]
preset = 'openai'
dialect = 'openai'
address = 'api.openai.com:443'
server_name = 'api.openai.com'
connect = 10
handshake = 10
[models.'openai/gpt-6-astra']
window = 272000
output = 32000
reasoning_item = 65536
head = 60
idle = 300
price_unit = 1000000
[models.'openai/gpt-6-luna']
window = 272000
output = 32000
reasoning_item = 65536
head = 60
idle = 300
price_unit = 1000000
[models.'openai/gpt-6.1-sol']
window = 272000
output = 32000
reasoning_item = 65536
head = 60
idle = 300
price_unit = 1000000
";
