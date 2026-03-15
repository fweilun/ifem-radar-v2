import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { login, setToken } from '../api';

export default function LoginPage() {
  const nav = useNavigate();
  const [account, setAccount] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const auth = await login(account, password);
      setToken(auth.access_token);
      nav('/surveys');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : '登入失敗');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="login-wrapper">
      <div className="login-card">
        <h1 className="login-title">
          <span className="logo-icon">📡</span>
          iFEM 雷達調查系統
        </h1>
        <form onSubmit={handleSubmit} className="login-form">
          <div className="form-group">
            <label>帳號</label>
            <input
              type="text"
              value={account}
              onChange={e => setAccount(e.target.value)}
              placeholder="請輸入帳號"
              autoComplete="username"
              required
            />
          </div>
          <div className="form-group">
            <label>密碼</label>
            <input
              type="password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              placeholder="請輸入密碼"
              autoComplete="current-password"
              required
            />
          </div>
          {error && <p className="error-msg">⚠ {error}</p>}
          <button type="submit" className="btn btn-primary btn-full" disabled={loading}>
            {loading ? '登入中…' : '登入'}
          </button>
        </form>
      </div>
    </div>
  );
}
