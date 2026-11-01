import json
import pathlib
import sys
from api_ast import collect_api
rules=json.loads(pathlib.Path(__file__).with_name('conventions.json').read_text(encoding='utf-8'))
sys.stdout.write(json.dumps(collect_api(json.load(sys.stdin),rules),ensure_ascii=False))
