import json
import pathlib
import sys
from workflow_ast import collect_workflow
from report_ast import collect_report
rules=json.loads(pathlib.Path(__file__).with_name('conventions.json').read_text(encoding='utf-8'))
payload=json.load(sys.stdin)
result=(collect_workflow if payload['stageId']=='workflow' else collect_report)(payload,rules)
sys.stdout.write(json.dumps(result,ensure_ascii=False))
