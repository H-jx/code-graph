"""One-time T06 fixture construction, before any business plugin exists.

This is an audit trail, not a test-time expected-output generator. Tests only read
the frozen files. Original source copies retain their Git provenance.
"""
import hashlib
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]
ORIGINAL = pathlib.Path('E:/Project/tbm/synes')
if (ROOT / 'fixtures/truth/manifest.json').exists():
    raise SystemExit('Frozen truth already exists. Changes require a separately reviewed old/new hash record.')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def write(path, content):
    target = ROOT / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding='utf-8', newline='\n')


def dump(path, value):
    write(path, json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def citation(path, needle):
    lines = (ROOT / path).read_text(encoding='utf-8').splitlines()
    matches = [i + 1 for i, line in enumerate(lines) if needle in line]
    assert matches, (path, needle)
    return {'path': path, 'startLine': matches[0], 'endLine': matches[0], 'text': needle}


originals = [
    'frontend/config/routes.ts',
    'frontend/src/locales/zh-CN/menu.ts',
    'frontend/src/modules/purchase/production-request/pages/ResearchPurchaseLedger/index.tsx',
    'frontend/src/modules/purchase/production-request/requests/queries/purchase-request.query.ts',
    'frontend/tests/e2e/specs/routes/other-purchase-ledger.spec.ts',
    'backend/src/modules/purchase-request/interface/graphql/resolvers/purchase-request.resolver.ts',
    'backend/src/modules/purchase-request/domain/value-objects/purchase-request-status.vo.ts',
    'backend/src/modules/purchase-request/domain/value-objects/purchase-request-status.vo.spec.ts',
    'backend/src/modules/purchase-request/application/workflows/purchase-request.workflow.ts',
    'backend/src/modules/purchase-request/infrastructure/entities/purchase-request.orm-entity.ts',
    'docs/260903-权限审计/03-采购.md',
]
provenance = []
for path in originals:
    data = (ORIGINAL / path).read_bytes()
    target = ROOT / 'fixtures/original/synes' / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    commit = subprocess.check_output(
        ['git', '-C', str(ORIGINAL), 'log', '-1', '--format=%H|%an|%aI', '--', path],
        text=True, encoding='utf-8').strip()
    provenance.append({'path': 'fixtures/original/synes/' + path,
                       'originalPath': path, 'sha256': sha(data), 'lastCommit': commit})
dump('fixtures/original/provenance.json', {
    'sourceRoot': str(ORIGINAL),
    'revision': subprocess.check_output(['git', '-C', str(ORIGINAL), 'rev-parse', 'HEAD'], text=True).strip(),
    'access': 'read-only; byte-identical copies', 'files': provenance,
})

for layout in ['a', 'b']:
    prefix = f'fixtures/page-location/{layout}'
    frontend = 'web' if layout == 'a' else 'client/ui'
    backend = 'api' if layout == 'a' else 'server/domain'
    config = {
        'schemaVersion': 1, 'projectId': f'acceptance.{layout}',
        'repositories': [{'id': 'front', 'root': frontend}, {'id': 'back', 'root': backend}],
        'selection': {},
        'scopes': [
            {'id': 'app.web', 'repositoryIds': ['front'], 'kind': 'application'},
            {'id': 'svc.erp', 'repositoryIds': ['back'], 'kind': 'service'},
        ],
        'services': [{'id': 'erp', 'scopeId': 'svc.erp', 'protocol': 'http', 'baseUrls': ['/erp']}],
        'dataSources': [{'id': ds, 'scopeId': 'svc.erp', 'dialect': 'sqlite'} for ds in ['ds.erp', 'ds.analytics']],
        'bindings': [{'id': 'erp-client', 'kind': 'clientToService', 'scopeId': 'app.web', 'from': 'http', 'to': 'erp'}],
    }
    dump(prefix + '/project.json', config)
    if layout == 'a':
        files = {
            'routes.ts': "import Ledger from './pages/Ledger';\nexport const router = { id: 'main', mode: 'hash' };\nexport const routes = [{ path: '/Other/purchaseLedger', alias: '/purchase', titleKey: 'purchase.title', component: Ledger }];\n",
            'messages.ts': "export const messages = { 'purchase.title': '采购台账 🧾' };\n",
            'pages/Ledger.tsx': "import { fetchLedger, fetchSummary, dynamicRequest } from '../requests';\n/** 采购台账：供应商筛选。 */\nexport function Ledger() {\n  async function query() { return fetchLedger(); }\n  async function summary() { return fetchSummary(); }\n  return <section><button onClick={query}>查询</button><button onClick={summary}>汇总</button><button onClick={dynamicRequest}>动态查询</button></section>;\n}\nexport default Ledger;\n",
            'requests.ts': "import { http } from './transport';\nexport function fetchLedger() { return http.get('/purchase/ledger'); }\nexport function fetchSummary() { return http.graphql(`query PurchaseSummary { purchaseSummary { count } }`); }\nexport function dynamicRequest() { return http.get('/purchase/' + globalThis.target); }\n",
            'transport.ts': "export const http = createClient({ baseURL: '/erp', serviceId: 'erp' });\n",
            'registrations.ts': "import { query } from './workers';\nexport const moduleInfo = { key: 'purchase', name: '采购模块' };\nscheduler.register('nightly-ledger', query);\nmessages.subscribe('purchase.refresh', query);\ncommands.register('refresh-ledger', query);\n",
            'workers.ts': "/** 采购台账缓存更新。 */\nexport function query() { return 'refresh'; }\n",
            'decoy/Ledger.tsx': "export function Ledger() { function query() { return 'decoy'; } return <button onClick={query}>查询</button>; }\n",
        }
        server_files = {
            'register.ts': "import { ledger } from './service';\nhttp.get('/purchase/ledger', ledger);\ngraphql.register('Query', 'purchaseSummary', ledger);\n",
            'service.ts': "export function ledger() { return erp.query(ledgerSql); }\nexport const ledgerSql = `SELECT * FROM purchase_detail WHERE tenant_id = :tenant`;\nexport const dynamicSql = 'SELECT * FROM ' + runtimeTable;\n",
            'models.ts': "export const erp = orm.connect('ds.erp');\nexport const analytics = orm.connect('ds.analytics');\nexport const Purchase = erp.model('purchase_detail', ['tenant_id', 'purchase_no', 'line_no']);\nexport const OtherPurchase = analytics.model('purchase_detail', ['tenant_id', 'purchase_no', 'line_no']);\n",
        }
    else:
        files = {
            'navigation.js': "export const router = { id: 'workspace', mode: 'history' };\nexport const screens = [{ url: '/Other/purchaseLedger', alias: '/purchase', labelKey: 'purchase.title', load: () => import('./views/Board') }];\n",
            'language.js': "export const messages = { 'purchase.title': '采购台账 🧾' };\n",
            'views/Board.tsx': "import { loadRows, loadStats, loadDynamic } from '../network';\n/** 采购台账：供应商筛选。 */\nexport function Board() {\n  async function search() { return loadRows(); }\n  async function statistics() { return loadStats(); }\n  return <main><button onClick={search}>查询</button><button onClick={statistics}>汇总</button><button onClick={loadDynamic}>动态查询</button></main>;\n}\nexport default Board;\n",
            'network.js': "import { http } from './client';\nexport function loadRows() { return http.get('/purchase/ledger'); }\nexport function loadStats() { return http.graphql(`query PurchaseSummary { purchaseSummary { count } }`); }\nexport function loadDynamic() { return http.get('/purchase/' + globalThis.target); }\n",
            'client.js': "export const http = createClient({ baseURL: '/erp', serviceId: 'erp' });\n",
            'startup.js': "import { search } from './jobs';\nexport const moduleInfo = { key: 'purchase', name: '采购模块' };\nscheduler.register('nightly-ledger', search);\nmessages.subscribe('purchase.refresh', search);\ncommands.register('refresh-ledger', search);\n",
            'jobs.js': "/** 采购台账缓存更新。 */\nexport function search() { return 'refresh'; }\n",
            'unrelated/Board.tsx': "export function Board() { function search() { return 'decoy'; } return <button onClick={search}>查询</button>; }\n",
        }
        server_files = {
            'expose.py': "from .queries import ledger\napp.get('/purchase/ledger')(ledger)\ngraphql.register('Query', 'purchaseSummary', ledger)\n",
            'queries.py': 'def ledger():\n    return erp.query("SELECT * FROM purchase_detail WHERE tenant_id = :tenant")\n\ndef dynamic_query(table):\n    return erp.query("SELECT * FROM " + table)\n',
            'models.py': "erp = orm.connect('ds.erp')\nanalytics = orm.connect('ds.analytics')\nPurchase = erp.model('purchase_detail', ['tenant_id', 'purchase_no', 'line_no'])\nOtherPurchase = analytics.model('purchase_detail', ['tenant_id', 'purchase_no', 'line_no'])\n",
        }
    for path, content in files.items():
        write(f'{prefix}/{frontend}/{path}', content)
    for path, content in server_files.items():
        write(f'{prefix}/{backend}/{path}', content)

write('fixtures/purchase-workflow/rules.md', '''# Audited acceptance model, not the synes business model

Source: NEW-PROJECT-LONG-TASK.md section 5; frozen before plugin development.
The state vocabulary below is synthetic. The original synes vocabulary is kept separately.

DRAFT -> SUBMITTED: submit, orderedQty > 0 AND supplierId present.
SUBMITTED -> APPROVED: approve, role == buyer AND approvalGranted.
SUBMITTED -> DRAFT: reject or withdraw, applicant == actor for withdraw.
APPROVED -> RECEIVING: receive, sum(POSTED, not-deleted receipt qty) > 0.
RECEIVING -> COMPLETED: finish, netReceived >= orderedQty AND all receipts valid.
APPROVED -> CANCELLED: cancel, netReceived == 0.
Documents purchase -> receipt -> return use ordered keys tenant_id, purchase_no, line_no.
UI pages: /purchase/approve precedes /Other/purchaseLedger, which precedes /purchase/receive.
The navigation /help is not a business successor.
Logging before saving is only a call order, with WORKFLOW_EVIDENCE_MISSING.
Conflict variant: a document allows finish at grossReceived >= orderedQty while code requires netReceived.
''')
write('fixtures/purchase-workflow/flow.ts', '''export function submit(order) {
  audit(order); // calling audit before save is not a business transition
  if (!(order.orderedQty > 0 && order.supplierId)) throw Error('invalid order');
  order.state = 'SUBMITTED';
  save(order);
}
export function approve(order, actor) {
  if (actor.role !== 'buyer' || !actor.approvalGranted) throw Error('not approved');
  order.state = 'APPROVED';
}
export function receive(order, receipts) {
  if (receipts.filter(r => r.status === 'POSTED' && !r.deleted).reduce((n, r) => n + r.qty, 0) > 0) order.state = 'RECEIVING';
}
export function finish(order) {
  if (order.netReceived >= order.orderedQty && order.allReceiptsValid) order.state = 'COMPLETED';
}
export function withdraw(order, actor) {
  if (order.state === 'SUBMITTED' && order.applicant === actor.id) order.state = 'DRAFT';
}
export function cancel(order) {
  if (order.state === 'APPROVED' && order.netReceived === 0) order.state = 'CANCELLED';
}
''')

write('fixtures/purchase-ledger/policy.md', '''# Independent arithmetic audit of the plan's proposed sample

This is a local audited acceptance model; it is not a claim about synes production accounting.
Grain: tenant_id + purchase_no + line_no, restricted to the requested tenant and APPROVED purchases.
Valid receipts/returns/invoices: status POSTED and deleted = 0. Aggregate each child table
at the grain BEFORE LEFT JOIN. No match is zero, not a reason to remove the detail.
grossReceived = receipt quantity sum; netReceived = grossReceived - return quantity sum;
outstanding = orderedQty - netReceived; purchaseAmountCents = orderedQty * unitPriceCents;
invoiceAmountCents = invoice amount sum. Supplier joins require tenant_id + supplier_id.
Two databases ds.erp and ds.analytics contain the same table names; only ds.erp is bound.
SQL runs only in the in-memory acceptance test. No business database is connected.
''')
write('fixtures/purchase-ledger/schema.sql', '''CREATE TABLE purchase_detail (
 tenant_id TEXT, purchase_no TEXT, line_no INTEGER, supplier_id TEXT,
 ordered_qty INTEGER, unit_price_cents INTEGER, status TEXT,
 PRIMARY KEY(tenant_id, purchase_no, line_no));
CREATE TABLE supplier (tenant_id TEXT, supplier_id TEXT, name TEXT, PRIMARY KEY(tenant_id,supplier_id));
CREATE TABLE receipt (id TEXT PRIMARY KEY, tenant_id TEXT, purchase_no TEXT, line_no INTEGER, qty INTEGER, status TEXT, deleted INTEGER,
 FOREIGN KEY(tenant_id,purchase_no,line_no) REFERENCES purchase_detail(tenant_id,purchase_no,line_no));
CREATE TABLE return_line (id TEXT PRIMARY KEY, tenant_id TEXT, purchase_no TEXT, line_no INTEGER, qty INTEGER, status TEXT, deleted INTEGER,
 FOREIGN KEY(tenant_id,purchase_no,line_no) REFERENCES purchase_detail(tenant_id,purchase_no,line_no));
CREATE TABLE invoice (id TEXT PRIMARY KEY, tenant_id TEXT, purchase_no TEXT, line_no INTEGER, amount_cents INTEGER, status TEXT, deleted INTEGER,
 FOREIGN KEY(tenant_id,purchase_no,line_no) REFERENCES purchase_detail(tenant_id,purchase_no,line_no));
''')
write('fixtures/purchase-ledger/data.sql', '''INSERT INTO purchase_detail VALUES
 ('A','P1',1,'S1',10,500,'APPROVED'), ('A','P1',2,'S1',4,2000,'APPROVED'),
 ('A','P2',1,'S2',3,700,'APPROVED'), ('A','P3',1,'S1',9,900,'CANCELLED'),
 ('B','P1',1,'S1',100,100,'APPROVED');
INSERT INTO supplier VALUES ('A','S1','本租户供应商'), ('A','S2','无收货供应商'), ('B','S1','其他租户供应商');
INSERT INTO receipt VALUES
 ('R1','A','P1',1,3,'POSTED',0), ('R2','A','P1',1,2,'POSTED',0),
 ('R3','A','P1',1,9,'CANCELLED',0), ('R4','A','P1',1,20,'POSTED',1),
 ('R5','A','P1',2,1,'POSTED',0), ('RB','B','P1',1,80,'POSTED',0);
INSERT INTO return_line VALUES ('T1','A','P1',1,1,'POSTED',0), ('T2','A','P1',1,99,'CANCELLED',0);
INSERT INTO invoice VALUES ('I1','A','P1',1,800,'POSTED',0), ('I2','A','P1',1,1200,'POSTED',0), ('I3','A','P1',1,9900,'POSTED',1);
''')
write('fixtures/purchase-ledger/correct.sql', '''WITH r AS (
 SELECT tenant_id,purchase_no,line_no,SUM(qty) AS qty FROM receipt
 WHERE status='POSTED' AND deleted=0 GROUP BY tenant_id,purchase_no,line_no
), t AS (
 SELECT tenant_id,purchase_no,line_no,SUM(qty) AS qty FROM return_line
 WHERE status='POSTED' AND deleted=0 GROUP BY tenant_id,purchase_no,line_no
), i AS (
 SELECT tenant_id,purchase_no,line_no,SUM(amount_cents) AS amount FROM invoice
 WHERE status='POSTED' AND deleted=0 GROUP BY tenant_id,purchase_no,line_no
)
SELECT p.tenant_id,p.purchase_no,p.line_no,s.name,
 COALESCE(r.qty,0) AS grossReceived,COALESCE(r.qty,0)-COALESCE(t.qty,0) AS netReceived,
 p.ordered_qty-COALESCE(r.qty,0)+COALESCE(t.qty,0) AS outstanding,
 p.ordered_qty*p.unit_price_cents AS purchaseAmountCents,COALESCE(i.amount,0) AS invoiceAmountCents
FROM purchase_detail p
LEFT JOIN supplier s ON s.tenant_id=p.tenant_id AND s.supplier_id=p.supplier_id
LEFT JOIN r ON r.tenant_id=p.tenant_id AND r.purchase_no=p.purchase_no AND r.line_no=p.line_no
LEFT JOIN t ON t.tenant_id=p.tenant_id AND t.purchase_no=p.purchase_no AND t.line_no=p.line_no
LEFT JOIN i ON i.tenant_id=p.tenant_id AND i.purchase_no=p.purchase_no AND i.line_no=p.line_no
WHERE p.tenant_id=:tenant AND p.status='APPROVED' ORDER BY p.purchase_no,p.line_no;
''')
write('fixtures/purchase-ledger/fanout.sql', '''SELECT p.purchase_no,p.line_no,SUM(r.qty),SUM(i.amount_cents)
FROM purchase_detail p
LEFT JOIN receipt r ON r.tenant_id=p.tenant_id AND r.purchase_no=p.purchase_no AND r.line_no=p.line_no AND r.status='POSTED' AND r.deleted=0
LEFT JOIN invoice i ON i.tenant_id=p.tenant_id AND i.purchase_no=p.purchase_no AND i.line_no=p.line_no AND i.status='POSTED' AND i.deleted=0
WHERE p.tenant_id=:tenant AND p.status='APPROVED' GROUP BY p.purchase_no,p.line_no;
''')
write('fixtures/purchase-ledger/where-child.sql', '''SELECT p.purchase_no,p.line_no,SUM(r.qty)
FROM purchase_detail p LEFT JOIN receipt r
ON r.tenant_id=p.tenant_id AND r.purchase_no=p.purchase_no AND r.line_no=p.line_no
WHERE p.tenant_id=:tenant AND p.status='APPROVED' AND r.status='POSTED' AND r.deleted=0
GROUP BY p.purchase_no,p.line_no;
''')

root = 'fixtures/original/synes/'
route = root + originals[0]
menu = root + originals[1]
page = root + originals[2]
workflow = root + originals[8]
policy = root + originals[10]

dump('fixtures/truth/scenario1.json', {
    'id': 'scenario1', 'origin': 'original-source-review',
    'facts': [
        {'id': 'real-route', 'value': '/Other/purchaseLedger', 'evidence': [citation(route, "path: '/Other/purchaseLedger'")]},
        {'id': 'real-component', 'value': 'ResearchPurchaseLedger', 'evidence': [citation(route, "component: '../modules/purchase/production-request/pages/ResearchPurchaseLedger/index'"), citation(page, 'const ResearchPurchaseLedger:')]},
        {'id': 'real-menu-title', 'value': '生产采购申请', 'evidence': [citation(menu, "'menu.other.purchaseLedger': '生产采购申请'")]},
        {'id': 'real-access', 'value': 'canAccessResearchPurchaseRequestLedger', 'evidence': [citation(route, "access: 'canAccessResearchPurchaseRequestLedger'")]},
        {'id': 'real-request-tab', 'value': 'PurchaseRequestTabContent', 'evidence': [citation(page, '<PurchaseRequestTabContent')]},
        {'id': 'real-graphql', 'value': 'purchaseRequests', 'evidence': [citation(root + originals[3], 'purchaseRequests(pagination:'), citation(root + originals[5], 'async purchaseRequests(')]},
        {'id': 'real-table', 'value': 'PurchaseRequest', 'evidence': [citation(root + originals[9], "@Entity({ tableName: 'PurchaseRequest' })")]},
    ],
    'forbidden': [
        {'id': 'no-other-to-procure', 'reason': 'same route name does not bind to another route', 'evidence': [citation(route, "path: '/Procure/purchaseLedger'"), citation(route, "path: '/Other/purchaseLedger'")]},
        {'id': 'no-invented-title', 'reason': 'the actual menu is not the generic 采购台账 title', 'evidence': [citation(menu, "'menu.other.purchaseLedger': '生产采购申请'")]},
    ],
})
dump('fixtures/truth/scenario2.json', {
    'id': 'scenario2', 'origin': 'original-source-review',
    'facts': [
        {'id': 'real-approval-plan', 'value': 'purchase-request.production-plan.approve', 'evidence': [citation(workflow, "ProductionPlanApprove = 'purchase-request.production-plan.approve'")]},
        {'id': 'real-approval-department', 'value': 'purchase-request.production-department.approve', 'evidence': [citation(workflow, "ProductionDepartmentApprove = 'purchase-request.production-department.approve'")]},
        {'id': 'real-approval-owner', 'value': 'purchase-request.project-owner.approve', 'evidence': [citation(workflow, "ProjectOwnerApprove = 'purchase-request.project-owner.approve'")]},
        {'id': 'real-withdraw', 'value': '撤销后回到待提交', 'evidence': [citation(policy, '撤销后，回到申请人的待提交阶段')]},
        {'id': 'real-close-guard', 'value': '其他原因需采购确认', 'evidence': [citation(policy, '当关闭原因是“其他”时，采购买办人员确认后，才能关闭')]},
    ],
    'auditedModel': {'source': 'fixtures/purchase-workflow/rules.md', 'states': ['DRAFT','SUBMITTED','APPROVED','RECEIVING','COMPLETED','CANCELLED'],
        'compositeKeys': ['tenant_id','purchase_no','line_no'],
        'guardEvidence': [citation('fixtures/purchase-workflow/flow.ts', 'order.orderedQty > 0 && order.supplierId'), citation('fixtures/purchase-workflow/flow.ts', 'order.netReceived >= order.orderedQty && order.allReceiptsValid')]},
})
dump('fixtures/truth/scenario3.json', {
    'id': 'scenario3', 'origin': 'independent-arithmetic-audit-of-designed-sample',
    'notProductionAccounting': True, 'tenant': 'A', 'grain': ['tenant_id','purchase_no','line_no'],
    'columns': ['tenant_id','purchase_no','line_no','supplier','grossReceived','netReceived','outstanding','purchaseAmountCents','invoiceAmountCents'],
    'rows': [['A','P1',1,'本租户供应商',5,4,6,5000,2000], ['A','P1',2,'本租户供应商',1,1,3,8000,0], ['A','P2',1,'无收货供应商',0,0,3,2100,0]],
    'totals': [6,5,12,15100,2000],
    'evidence': [citation('fixtures/purchase-ledger/policy.md', 'Grain:'), citation('fixtures/purchase-ledger/data.sql', "('A','P1',1,'S1',10,500,'APPROVED')")],
    'mutants': {'fanout.sql': {'firstGrossReceived': 10, 'firstInvoiceCents': 4000}, 'where-child.sql': {'rowCount': 2}},
})
dump('fixtures/truth/negative.json', {
    'id': 'negative',
    'cases': [
        {'id': 'X01', 'rule': 'same name in decoy file must not bind to registered page', 'evidence': [citation('fixtures/page-location/a/web/decoy/Ledger.tsx', 'function Ledger')]},
        {'id': 'X04', 'rule': 'dynamic request target has DYNAMIC_TARGET and no confirmed endpoint', 'evidence': [citation('fixtures/page-location/a/web/requests.ts', "'/purchase/' + globalThis.target")]},
        {'id': 'S2-05', 'rule': 'audit before save is calls, not precedes', 'evidence': [citation('fixtures/purchase-workflow/flow.ts', 'audit(order);')]},
        {'id': 'S2-06', 'rule': 'conflicting gross/net guard retains both bases and CONFLICTING_EVIDENCE', 'evidence': [citation('fixtures/purchase-workflow/rules.md', 'Conflict variant:')]},
        {'id': 'S3-fanout', 'rule': 'join children before aggregation duplicates amounts', 'evidence': [citation('fixtures/purchase-ledger/fanout.sql', 'LEFT JOIN invoice')]},
        {'id': 'S3-left', 'rule': 'WHERE child status drops no-receipt row', 'evidence': [citation('fixtures/purchase-ledger/where-child.sql', "AND r.status='POSTED'")]},
        {'id': 'real-stale-test-access', 'rule': 'historical e2e comment and current route differ; preserve conflict, not override', 'evidence': [citation(root + originals[4], '权限 access：canCreateRootProcureRequisition'), citation(route, "access: 'canAccessResearchPurchaseRequestLedger'")]},
    ],
})

# Frozen frontend expectations use logical identity, never plugin output.
layouts = []
for layout, frontend, route_file, page_file, component, handler, wrapper, client, registration, workers in [
    ('a','web','routes.ts','pages/Ledger.tsx','Ledger','query','fetchLedger','transport.ts','registrations.ts','workers.ts'),
    ('b','client/ui','navigation.js','views/Board.tsx','Board','search','loadRows','client.js','startup.js','jobs.js'),
]:
    prefix = f'fixtures/page-location/{layout}/{frontend}/'
    request_file = 'requests.ts' if layout == 'a' else 'network.js'
    message_file = 'messages.ts' if layout == 'a' else 'language.js'
    worker = 'query' if layout == 'a' else 'search'
    def entity(key, kind, identity, file, needle, **attributes):
        return {'key': key, 'kind': kind, 'scopeId': 'app.web', 'identity': identity,
                'attributes': attributes, 'evidence': [citation(prefix + file, needle)]}
    def local(path, **extra):
        return dict(repositoryId='front', path=path, scopeId='app.web', **extra)
    entities = [
        entity('route', 'route', dict(scopeId='app.web',routerId='main' if layout=='a' else 'workspace',mode='hash' if layout=='a' else 'history',declaredPath='/Other/purchaseLedger'), route_file, '/Other/purchaseLedger', declaredPath='/Other/purchaseLedger', alias='/purchase'),
        entity('page', 'component', local(page_file, lexicalPath=component), page_file, f'function {component}'),
        entity('title', 'title', local(message_file, lexicalPath='purchase.title'), message_file, '采购台账 🧾', text='采购台账 🧾', i18nKey='purchase.title'),
        entity('handler', 'operation', local(page_file, lexicalPath=component+'.'+handler, normalizedSignature='()'), page_file, f'function {handler}'),
        entity('event', 'event', local(page_file, lexicalPath=component+'.onClick.'+handler), page_file, 'onClick={'+handler+'}', event='onClick'),
        entity('wrapper', 'operation', local(request_file, lexicalPath=wrapper, normalizedSignature='()'), request_file, f'function {wrapper}'),
        entity('request', 'request', dict(scopeId='app.web',requestKey=request_file+'#'+wrapper), request_file, "http.get('/purchase/ledger')", method='GET',path='/purchase/ledger',baseURL='/erp',serviceId='erp'),
        entity('worker', 'operation', local(workers, lexicalPath=worker, normalizedSignature='()'), workers, f'function {worker}'),
        entity('module', 'module', dict(scopeId='app.web',semanticKey='purchase'), registration, 'moduleInfo', name='采购模块'),
        entity('comment', 'comment', local(page_file, lexicalPath=component+'.doc'), page_file, '采购台账：供应商筛选', text='采购台账：供应商筛选。'),
    ]
    for entry in ['nightly-ledger','purchase.refresh','refresh-ledger']:
        entities.append(entity('entry:'+entry, 'entrypoint', dict(scopeId='app.web',semanticKey=entry), registration, "'"+entry+"'"))
    relations = []
    for kind, source, target, file, needle in [
        ('renders','route','page',route_file,'/Other/purchaseLedger'),
        ('describes','title','route',route_file,'purchase.title'),
        ('contains','page','handler',page_file,f'function {handler}'),
        ('triggers','event','handler',page_file,'onClick={'+handler+'}'),
        ('calls','handler','wrapper',page_file,f'return {wrapper}()'),
        ('requests','wrapper','request',request_file,"http.get('/purchase/ledger')"),
        ('describes','comment','page',page_file,'采购台账：供应商筛选'),
    ]:
        relations.append({'kind':kind,'from':source,'to':target,'qualifiers':{},'evidence':[citation(prefix+file,needle)]})
    for entry in ['nightly-ledger','purchase.refresh','refresh-ledger']:
        relations.append({'kind':'handles','from':'entry:'+entry,'to':'worker','qualifiers':{},'evidence':[citation(prefix+registration,"'"+entry+"'")]})
    layouts.append({
        'layout': layout, 'projectId': f'acceptance.{layout}', 'scopeId': 'app.web',
        'routerId': 'main' if layout == 'a' else 'workspace', 'mode': 'hash' if layout == 'a' else 'history',
        'routePath': '/Other/purchaseLedger', 'alias': '/purchase',
        'routeFile': route_file, 'pageFile': page_file, 'component': component,
        'handler': handler, 'wrapper': wrapper, 'requestFile': request_file,
        'title': '采购台账 🧾', 'titleKey': 'purchase.title', 'messageFile': message_file,
        'http': {'method': 'GET','path': '/purchase/ledger','serviceId': 'erp','baseURL':'/erp'},
        'graphql': {'type':'query','name':'PurchaseSummary','field':'purchaseSummary'},
        'entrypoints': ['nightly-ledger','purchase.refresh','refresh-ledger'], 'registrationFile': registration,
        'workerFile': workers, 'worker': worker,
        'expectedEntities': entities, 'expectedRelations': relations,
        'forbiddenRelations': [{'kind':'renders','from':'route','toIdentity':local('decoy/Ledger.tsx' if layout=='a' else 'unrelated/Board.tsx',lexicalPath=component),'toKind':'component','qualifiers':{},'evidence':[citation(prefix+route_file,'/Other/purchaseLedger')]}],
        'evidence': [citation(prefix + route_file, '/Other/purchaseLedger'), citation(prefix + page_file, f'function {component}'), citation(prefix + request_file, "http.get('/purchase/ledger')"), citation(prefix + message_file, '采购台账 🧾')],
        'classification': 'synthetic-source-reviewed-smoke; not production accuracy',
    })
dump('fixtures/truth/frontend.json', {'id': 'frontend', 'layouts': layouts})

write('fixtures/truth/review.md', '''# T06 source review — 2026-10-28 Asia/Shanghai

Reviewer: Codex, source inspection performed before implementing the business collectors.
Method: read original registration, i18n, page, GraphQL, state/workflow, ORM and permissions materials;
compare independent files and the original pre-existing tests. No collector output was used.
Original authors/revisions per file are preserved in fixtures/original/provenance.json.
This record does not claim human sign-off or that original authors reviewed this truth package.

Scope: scenario1/2 original facts are source-declared implementation/document facts, not an assertion
that business requirements are correct. The route's actual menu is 生产采购申请. The historical e2e
permission comment differs from current routes.ts; both are retained as a conflict, not silently merged.
GraphQL shared transport does not prove any other field/resolver relationship. Unknown database instance
bindings and numeric production rules remain gaps. The synthetic DRAFT/etc vocabulary is separate.

The A/B TS/TSX and JS/TSX+Python fixtures are explicitly smoke models. Their static source was read
against frozen frontend expectations. No real-project accuracy percentage is assigned to them.
The arithmetic sample has a separate audit: integer-cent calculation from individual input records,
compared with in-memory SQLite preaggregation. Fanout and WHERE-child mutants must disagree.
The audited arithmetic validates only the stated acceptance policy, never production accounting.

Truth change policy: immutable baseline. Any change requires a new review reason and old/new truthHash;
tests must fail on changed source/hash, missing citations, changed expected or weakened negative cases.
Candidate tools write only owned work/agent run revisions; they cannot choose a truth output path.
''')

files = {}
for folder in ['fixtures/original','fixtures/page-location','fixtures/purchase-workflow','fixtures/purchase-ledger','fixtures/truth']:
    for path in sorted((ROOT / folder).rglob('*')):
        if path.is_file() and path.name not in ['.gitkeep','manifest.json']:
            files[path.relative_to(ROOT).as_posix()] = sha(path.read_bytes())
canonical = json.dumps(files, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
dump('fixtures/truth/manifest.json', {
    'schemaVersion': 1, 'frozenAt': '2026-10-28 Asia/Shanghai',
    'sourceRevision': '6f47bdbf901f66e59909f8e24edd2e41a1b4f379',
    'review': 'fixtures/truth/review.md', 'files': files, 'truthHash': sha(canonical),
    'classification': {'original': 'source-reviewed implementation facts', 'layouts': 'synthetic smoke', 'numeric': 'independently audited designed sample'},
})
print(json.dumps({'files': len(files), 'truthHash': sha(canonical)}, indent=2))
