import { loadRows, loadStats, loadDynamic } from '../network';
/** 采购台账：供应商筛选。 */
export function Board() {
  async function search() { return loadRows(); }
  async function statistics() { return loadStats(); }
  return <main><button onClick={search}>查询</button><button onClick={statistics}>汇总</button><button onClick={loadDynamic}>动态查询</button></main>;
}
export default Board;
