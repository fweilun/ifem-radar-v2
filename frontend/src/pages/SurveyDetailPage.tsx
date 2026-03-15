import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { clearToken, deletePhoto, getSurvey, listPhotos, photoUrl, uploadPhotos } from '../api';
import type { PhotoRecord, SurveyRecord } from '../types';
import { getCategoryLabel, getFieldDefForCategory } from '../types';

export default function SurveyDetailPage() {
  const { id } = useParams<{ id: string }>();
  const nav = useNavigate();

  const [survey, setSurvey] = useState<SurveyRecord | null>(null);
  const [photos, setPhotos] = useState<PhotoRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  // Photo upload state
  const [uploading, setUploading] = useState(false);
  const [uploadError, setUploadError] = useState('');
  const [lightbox, setLightbox] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleAuthError = useCallback(
    (err: unknown) => {
      if (err instanceof Error && err.message.includes('401')) {
        clearToken();
        nav('/login');
        return true;
      }
      return false;
    },
    [nav]
  );

  const loadData = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    setError('');
    try {
      const [s, p] = await Promise.all([getSurvey(id), listPhotos(id)]);
      setSurvey(s);
      setPhotos(p);
    } catch (err) {
      if (!handleAuthError(err)) {
        setError(err instanceof Error ? err.message : '載入失敗');
      }
    } finally {
      setLoading(false);
    }
  }, [id, handleAuthError]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files ?? []);
    if (!files.length || !id) return;
    setUploading(true);
    setUploadError('');
    try {
      await uploadPhotos(id, files);
      const p = await listPhotos(id);
      setPhotos(p);
    } catch (err) {
      if (!handleAuthError(err)) {
        setUploadError(err instanceof Error ? err.message : '上傳失敗');
      }
    } finally {
      setUploading(false);
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  const handleDeletePhoto = async (photoId: string) => {
    if (!confirm('確定刪除這張照片？')) return;
    try {
      await deletePhoto(photoId);
      setPhotos(prev => prev.filter(p => p.id !== photoId));
    } catch (err) {
      if (!handleAuthError(err)) {
        alert(err instanceof Error ? err.message : '刪除失敗');
      }
    }
  };

  const formatDate = (iso?: string) => {
    if (!iso) return '—';
    return new Date(iso).toLocaleString('zh-TW');
  };

  if (loading) {
    return (
      <div className="page">
        <header className="topbar">
          <span className="topbar-title">📡 iFEM 雷達調查系統</span>
          <button className="btn btn-secondary" onClick={() => nav('/surveys')}>← 返回列表</button>
        </header>
        <main className="container"><p className="loading-text">載入中…</p></main>
      </div>
    );
  }

  if (error || !survey) {
    return (
      <div className="page">
        <header className="topbar">
          <span className="topbar-title">📡 iFEM 雷達調查系統</span>
          <button className="btn btn-secondary" onClick={() => nav('/surveys')}>← 返回列表</button>
        </header>
        <main className="container">
          <p className="error-msg">⚠ {error || '找不到此記錄'}</p>
        </main>
      </div>
    );
  }

  const fieldDef = getFieldDefForCategory(survey.category) ?? { dict: [], list: [] };

  return (
    <div className="page">
      {/* ── Top bar ── */}
      <header className="topbar">
        <span className="topbar-title">📡 iFEM 雷達調查系統</span>
        <button className="btn btn-secondary" onClick={() => nav('/surveys')}>
          ← 返回列表
        </button>
      </header>

      <main className="container">
        <h2 className="section-title">
          調查詳情
          <span className="category-badge" style={{ marginLeft: '0.75rem', fontSize: '1rem' }}>
            {getCategoryLabel(survey.category)}
          </span>
        </h2>

        {/* ── 基本資訊 ── */}
        <section className="detail-card">
          <h3 className="card-title">📋 基本資訊</h3>
          <div className="detail-grid">
            <div className="detail-item">
              <span className="detail-label">起點</span>
              <span className="detail-value">{survey.start_point}</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">迄點</span>
              <span className="detail-value">{survey.end_point}</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">方向</span>
              <span className="detail-value">{survey.orientation}</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">距離</span>
              <span className="detail-value">{survey.distance} m</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">頂距</span>
              <span className="detail-value">{survey.top_distance}</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">建立時間</span>
              <span className="detail-value">{formatDate(survey.created_at)}</span>
            </div>
          </div>
        </section>

        {/* ── 量測數據 ── */}
        {fieldDef.dict.length > 0 && (
          <section className="detail-card">
            <h3 className="card-title">📏 量測數據</h3>
            <div className="detail-grid">
              {renderDictFields(survey, fieldDef.dict)}
            </div>
          </section>
        )}

        {/* ── 缺失狀況 ── */}
        {fieldDef.list.length > 0 && (
          <section className="detail-card">
            <h3 className="card-title">⚠ 缺失狀況</h3>
            <div className="chip-group">
              {fieldDef.list.map(item => {
                const active = survey.details.issues?.includes(item);
                return (
                  <span key={item} className={`issue-chip readonly${active ? ' active' : ''}`}>
                    {item}
                  </span>
                );
              })}
            </div>
          </section>
        )}

        {/* ── 備註 ── */}
        {survey.remarks && (
          <section className="detail-card">
            <h3 className="card-title">📝 備註</h3>
            <p className="remark-text">{survey.remarks}</p>
          </section>
        )}

        {/* ── 照片 ── */}
        <section className="detail-card">
          <div className="card-header-row">
            <h3 className="card-title">📷 照片（{photos.length}）</h3>
            <label className={`btn btn-primary btn-sm${uploading ? ' disabled' : ''}`}>
              {uploading ? '上傳中…' : '＋ 上傳照片'}
              <input
                ref={fileInputRef}
                type="file"
                accept="image/*"
                multiple
                style={{ display: 'none' }}
                onChange={handleUpload}
                disabled={uploading}
              />
            </label>
          </div>

          {uploadError && <p className="error-msg">⚠ {uploadError}</p>}

          {photos.length === 0 ? (
            <p className="empty-text">尚無照片，點擊上方按鈕上傳。</p>
          ) : (
            <div className="photo-grid">
              {photos.map(p => (
                <div key={p.id} className="photo-item">
                  <img
                    src={photoUrl(p.id)}
                    alt={p.filename}
                    loading="lazy"
                    onClick={() => setLightbox(photoUrl(p.id))}
                  />
                  <div className="photo-overlay">
                    <span className="photo-name">{p.filename}</span>
                    <button
                      className="btn btn-danger btn-sm"
                      onClick={() => handleDeletePhoto(p.id)}
                    >
                      刪除
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </main>

      {/* ── Lightbox ── */}
      {lightbox && (
        <div className="lightbox" onClick={() => setLightbox(null)}>
          <img src={lightbox} alt="preview" onClick={e => e.stopPropagation()} />
          <button className="lightbox-close" onClick={() => setLightbox(null)}>✕</button>
        </div>
      )}
    </div>
  );
}

// Helper: render dict fields from SurveyDetails by label
function renderDictFields(survey: SurveyRecord, labels: string[]) {
  const d = survey.details;
  const labelToValue: Record<string, string> = {
    直徑: d.diameter != null ? `${d.diameter}` : '—',
    長: d.length != null ? `${d.length}` : '—',
    長度: d.length != null ? `${d.length}` : '—',
    寬: d.width != null ? `${d.width}` : '—',
    寬度: d.width != null ? `${d.width}` : '—',
    凸出: d.protrusion != null ? `${d.protrusion}` : '—',
    淤積: d.siltation_depth != null ? `${d.siltation_depth} cm` : '—',
    根: d.crossing_pipe_count != null ? `${d.crossing_pipe_count}` : '—',
    跌降: d.change_of_area?.height != null ? `${d.change_of_area.height}` : '—',
    變化: d.change_of_area?.change_height != null ? `${d.change_of_area.change_height}` : '—',
  };

  return labels.map(label => (
    <div key={label} className="detail-item">
      <span className="detail-label">{label}</span>
      <span className="detail-value">{labelToValue[label] ?? '—'}</span>
    </div>
  ));
}
