import json
import pathlib
import sys
from database_ast import collect_database
rules=json.loads(pathlib.Path(__file__).with_name('conventions.json').read_text(encoding='utf-8'))
sys.stdout.write(json.dumps(collect_database(json.load(sys.stdin),rules),ensure_ascii=False))
