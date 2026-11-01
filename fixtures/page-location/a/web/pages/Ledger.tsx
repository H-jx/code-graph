import { fetchLedger, fetchSummary, dynamicRequest } from '../requests';
/** 采购台账：供应商筛选。 */
export function Ledger() {
  async function query() { return fetchLedger(); }
  async function summary() { return fetchSummary(); }
  return <section><button onClick={query}>查询</button><button onClick={summary}>汇总</button><button onClick={dynamicRequest}>动态查询</button></section>;
}
export default Ledger;
