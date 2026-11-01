"""Read-only T06 integrity check and arithmetic oracle; no plugin code is imported."""
import ast
import hashlib
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]


def load(path):
    return json.loads((ROOT / path).read_text(encoding='utf-8'))


def verify():
    manifest = load('fixtures/truth/manifest.json')
    files = manifest['files']
    for path, expected in files.items():
        assert hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == expected, path
    digest = hashlib.sha256(json.dumps(files, ensure_ascii=False, sort_keys=True,
                                      separators=(',', ':')).encode()).hexdigest()
    assert digest == manifest['truthHash']
    # Independent input parsing: literal tuples -> keyed integer accumulation.
    # No SQL engine, collector or Rust result is used to derive the oracle.
    sql = (ROOT / 'fixtures/purchase-ledger/data.sql').read_text(encoding='utf-8')
    tables = {}
    for name, values in re.findall(r'INSERT INTO (\w+) VALUES\s*(.*?);', sql, re.S):
        tables[name] = ast.literal_eval('[' + values + ']')
    children = {}
    for name in ['receipt', 'return_line', 'invoice']:
        groups = {}
        for _, tenant, number, line, amount, status, deleted in tables[name]:
            if status == 'POSTED' and not deleted:
                key = (tenant, number, line)
                groups[key] = groups.get(key, 0) + amount
        children[name] = groups
    suppliers = {(tenant, key): name for tenant, key, name in tables['supplier']}
    rows = []
    for tenant, number, line, supplier, ordered, price, status in tables['purchase_detail']:
        if tenant != 'A' or status != 'APPROVED':
            continue
        key = (tenant, number, line)
        gross = children['receipt'].get(key, 0)
        net = gross - children['return_line'].get(key, 0)
        rows.append([tenant, number, line, suppliers[(tenant, supplier)], gross, net,
                     ordered - net, ordered * price, children['invoice'].get(key, 0)])
    rows.sort(key=lambda r: (r[1], r[2]))
    totals = [sum(row[column] for row in rows) for column in range(4, 9)]
    truth = load('fixtures/truth/scenario3.json')
    assert rows == truth['rows'], (rows, truth['rows'])
    assert totals == truth['totals'], (totals, truth['totals'])
    return {'truthHash': digest, 'verifiedFiles': len(files), 'rows': rows, 'totals': totals,
            'arithmetic': 'Python keyed integer accumulation; no SQL or collector',
            'productionAccuracyClaimed': False}


if __name__ == '__main__':
    print(json.dumps(verify(), ensure_ascii=False, indent=2))
