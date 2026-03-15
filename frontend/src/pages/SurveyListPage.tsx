import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { clearToken, listSurveys } from '../api';
import type { SurveyQueryParams, SurveyRecord } from '../types';
import { CATEGORY_LABELS, CATEGORY_VALUES, getCategoryLabel } from '../types';

const ORIENTATIONS = ['上', '下', '左', '右'];

export default function SurveyListPage() {
  const nav = useNavigate();
  const [surveys, setSurveys] = useState<SurveyRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const [filters, setFilters] = useState<SurveyQueryParams>({
    category: '',
    start_point: '',
    end_point: '',
    created_from: '',
    created_to: '',
  });

  const load = async () => {
    setLoading(true);
    setError('');
    try {
      const data = await listSurveys(filters);
      setSurveys(data);
    } catch (err: unknown) {
      if (err instanceof Error && err.message.includes('401')) {
        clearToken();
        nav('/login');
      } else {
        setError(err instanceof Error ? err.message : '載入失敗');
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const handleFilterChange = (key: keyof SurveyQueryParams, value: string) => {
    setFilters(prev => ({ ...prev, [key]: value }));
  };

  const handleLogout = () => {
    clearToken();
    nav('/login');
  };

  const formatDate = (iso?: string) => {
    if (!iso) return '—';
    return new Date(iso).toLocaleString('zh-TW', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <div className="page">
      {/* ── Top bar ── */}
      <header className="topbar">
        <span className="topbar-title">📡 iFEM 雷達調查系統</span>
        <div className="topbar-actions">
          <button className="btn btn-primary" onClick={() => nav('/surveys/new')}>
            ＋ 新增調查
          </button>
          <button className="btn btn-secondary" onClick={handleLogout}>
            登出
          </button>
        </div>
      </header>

      <main className="container">
        {/* ── Filters ── */}
        <section className="filter-panel">
          <h2 className="section-title">🔍 篩選條件</h2>
          <div className="filter-grid">
            <div className="form-group">
              <label>類別</label>
              <select
                value={filters.category ?? ''}
                onChange={e => handleFilterChange('category', e.target.value)}
              >
                <option value="">全部</option>
                {CATEGORY_VALUES.map(c => (
                  <option key={c} value={c}>
                    {CATEGORY_LABELS[c]}
                  </option>
                ))}
              </select>
            </div>
            <div className="form-group">
              <label>起點</label>
              <input
                type="text"
                value={filters.start_point ?? ''}
                onChange={e => handleFilterChange('start_point', e.target.value)}
                placeholder="起點"
              />
            </div>
            <div className="form-group">
              <label>迄點</label>
              <input
                type="text"
                value={filters.end_point ?? ''}
                onChange={e => handleFilterChange('end_point', e.target.value)}
                placeholder="迄點"
              />
            </div>
            <div className="form-group">
              <label>建立日期（從）</label>
              <input
                type="datetime-local"
                value={filters.created_from ?? ''}
                onChange={e => handleFilterChange('created_from', e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>建立日期（至）</label>
              <input
                type="datetime-local"
                value={filters.created_to ?? ''}
                onChange={e => handleFilterChange('created_to', e.target.value)}
              />
            </div>
          </div>
          <button className="btn btn-primary" onClick={load} disabled={loading}>
            {loading ? '搜尋中…' : '搜尋'}
          </button>
        </section>

        {/* ── Error ── */}
        {error && <p className="error-msg">⚠ {error}</p>}

        {/* ── Table ── */}
        <section className="table-section">
          <div className="table-header">
            <h2 className="section-title">調查記錄（{surveys.length} 筆）</h2>
          </div>
          {loading ? (
            <p className="loading-text">載入中…</p>
          ) : surveys.length === 0 ? (
            <p className="empty-text">無資料，請調整篩選條件或新增調查。</p>
          ) : (
            <div className="table-wrapper">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>類別</th>
                    <th>起點</th>
                    <th>迄點</th>
                    <th>方向</th>
                    <th>距離(m)</th>
                    <th>頂距</th>
                    <th>建立時間</th>
                    <th>操作</th>
                  </tr>
                </thead>
                <tbody>
                  {surveys.map(s => (
                    <tr key={s.id}>
                      <td>
                        <span className="category-badge">
                          {getCategoryLabel(s.category)}
                        </span>
                      </td>
                      <td>{s.start_point}</td>
                      <td>{s.end_point}</td>
                      <td>{s.orientation}</td>
                      <td>{s.distance}</td>
                      <td>{s.top_distance}</td>
                      <td>{formatDate(s.created_at)}</td>
                      <td>
                        <button
                          className="btn btn-sm btn-outline"
                          onClick={() => nav(`/surveys/${s.id}`)}
                        >
                          查看
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      </main>
    </div>
  );
}

export { ORIENTATIONS };
