import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { v4 as uuidv4 } from 'uuid';
import { clearToken, createSurvey } from '../api';
import type { CreateSurveyRequest, SurveyCategory, SurveyDetails } from '../types';
import { CATEGORY_LABELS, CATEGORY_VALUES, DICT_FIELD_MAP, FIELD_MAP } from '../types';

const ORIENTATIONS = ['上', '下', '左', '右'];

const NUMERIC_FIELDS = new Set(['直徑', '長', '寬', '凸出', '淤積', '根', '寬度', '長度', '跌降', '變化']);

export default function CreateSurveyPage() {
  const nav = useNavigate();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  // ── Base fields ──────────────────────────────────────────────────────────
  const [startPoint, setStartPoint] = useState('');
  const [endPoint, setEndPoint] = useState('');
  const [orientation, setOrientation] = useState(ORIENTATIONS[0]);
  const [distance, setDistance] = useState('');
  const [topDistance, setTopDistance] = useState('');
  const [category, setCategory] = useState<SurveyCategory>('connecting_pipe');
  const [remarks, setRemarks] = useState('');

  // ── Dynamic detail fields ─────────────────────────────────────────────────
  // dict fields: { labelName -> string value }
  const [dictValues, setDictValues] = useState<Record<string, string>>({});
  // list fields: selected set
  const [selectedIssues, setSelectedIssues] = useState<Set<string>>(new Set());

  const fieldDef = FIELD_MAP[category];

  const handleCategoryChange = (cat: SurveyCategory) => {
    setCategory(cat);
    setDictValues({});
    setSelectedIssues(new Set());
  };

  const toggleIssue = (item: string) => {
    setSelectedIssues(prev => {
      const next = new Set(prev);
      next.has(item) ? next.delete(item) : next.add(item);
      return next;
    });
  };

  const buildDetails = (): SurveyDetails => {
    const details: SurveyDetails = {};

    // Map dict labels to SurveyDetails fields
    for (const label of fieldDef.dict) {
      const fieldName = DICT_FIELD_MAP[label];
      const raw = dictValues[label];
      if (!raw || raw.trim() === '') continue;
      const num = parseFloat(raw);

      if (fieldName === 'drop_height' || fieldName === 'section_change_value') {
        // section_change maps to change_of_area
        if (!details.change_of_area) {
          details.change_of_area = { width: 0, height: 0, change_width: 0, change_height: 0 };
        }
        if (fieldName === 'drop_height') details.change_of_area.height = num;
        if (fieldName === 'section_change_value') details.change_of_area.change_height = num;
      } else if (fieldName) {
        (details as Record<string, number | undefined>)[fieldName] = isNaN(num) ? undefined : num;
      }
    }

    if (selectedIssues.size > 0) {
      details.issues = Array.from(selectedIssues);
    }

    return details;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setSaving(true);

    const details = buildDetails();

    const payload: CreateSurveyRequest = {
      id: uuidv4(),
      start_point: startPoint.trim(),
      end_point: endPoint.trim(),
      orientation,
      distance: parseFloat(distance) || 0,
      top_distance: topDistance.trim(),
      category,
      details,
      remarks: remarks.trim() || undefined,
    };

    try {
      await createSurvey(payload);
      nav('/surveys');
    } catch (err: unknown) {
      if (err instanceof Error && err.message.includes('401')) {
        clearToken();
        nav('/login');
      } else {
        setError(err instanceof Error ? err.message : '儲存失敗');
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="page">
      <header className="topbar">
        <span className="topbar-title">📡 iFEM 雷達調查系統</span>
        <button className="btn btn-secondary" onClick={() => nav('/surveys')}>
          ← 返回列表
        </button>
      </header>

      <main className="container">
        <h2 className="section-title">＋ 新增調查記錄</h2>

        <form onSubmit={handleSubmit} className="survey-form">
          {/* ── 基本資訊 ── */}
          <fieldset className="form-section">
            <legend>基本資訊</legend>
            <div className="form-grid">
              <div className="form-group">
                <label>起點 *</label>
                <input
                  type="text"
                  value={startPoint}
                  onChange={e => setStartPoint(e.target.value)}
                  placeholder="例：MH-001"
                  required
                />
              </div>
              <div className="form-group">
                <label>迄點 *</label>
                <input
                  type="text"
                  value={endPoint}
                  onChange={e => setEndPoint(e.target.value)}
                  placeholder="例：MH-002"
                  required
                />
              </div>
              <div className="form-group">
                <label>方向 *</label>
                <select value={orientation} onChange={e => setOrientation(e.target.value)}>
                  {ORIENTATIONS.map(o => (
                    <option key={o} value={o}>{o}</option>
                  ))}
                </select>
              </div>
              <div className="form-group">
                <label>距離 (m) *</label>
                <input
                  type="number"
                  step="0.01"
                  value={distance}
                  onChange={e => setDistance(e.target.value)}
                  placeholder="0.00"
                  required
                />
              </div>
              <div className="form-group">
                <label>頂距 *</label>
                <input
                  type="text"
                  value={topDistance}
                  onChange={e => setTopDistance(e.target.value)}
                  placeholder="例：>0"
                  required
                />
              </div>
            </div>
          </fieldset>

          {/* ── 類別 ── */}
          <fieldset className="form-section">
            <legend>缺失類別</legend>
            <div className="category-grid">
              {CATEGORY_VALUES.map(cat => (
                <button
                  key={cat}
                  type="button"
                  className={`category-chip${category === cat ? ' active' : ''}`}
                  onClick={() => handleCategoryChange(cat)}
                >
                  {CATEGORY_LABELS[cat]}
                </button>
              ))}
            </div>
          </fieldset>

          {/* ── 動態欄位 ── */}
          {fieldDef.dict.length > 0 && (
            <fieldset className="form-section">
              <legend>量測數據</legend>
              <div className="form-grid">
                {fieldDef.dict.map(label => (
                  <div key={label} className="form-group">
                    <label>{label}</label>
                    <input
                      type={NUMERIC_FIELDS.has(label) ? 'number' : 'text'}
                      step="any"
                      value={dictValues[label] ?? ''}
                      onChange={e =>
                        setDictValues(prev => ({ ...prev, [label]: e.target.value }))
                      }
                      placeholder={NUMERIC_FIELDS.has(label) ? '數字' : label}
                    />
                  </div>
                ))}
              </div>
            </fieldset>
          )}

          {fieldDef.list.length > 0 && (
            <fieldset className="form-section">
              <legend>缺失狀況（可複選）</legend>
              <div className="chip-group">
                {fieldDef.list.map(item => (
                  <button
                    key={item}
                    type="button"
                    className={`issue-chip${selectedIssues.has(item) ? ' active' : ''}`}
                    onClick={() => toggleIssue(item)}
                  >
                    {item}
                  </button>
                ))}
              </div>
            </fieldset>
          )}

          {/* ── 備註 ── */}
          <fieldset className="form-section">
            <legend>備註</legend>
            <textarea
              className="full-textarea"
              value={remarks}
              onChange={e => setRemarks(e.target.value)}
              rows={3}
              placeholder="選填備註"
            />
          </fieldset>

          {error && <p className="error-msg">⚠ {error}</p>}

          <div className="form-actions">
            <button type="button" className="btn btn-secondary" onClick={() => nav('/surveys')}>
              取消
            </button>
            <button type="submit" className="btn btn-primary" disabled={saving}>
              {saving ? '儲存中…' : '儲存記錄'}
            </button>
          </div>
        </form>
      </main>
    </div>
  );
}
