import type {
  AuthBody,
  CreateSurveyRequest,
  PhotoRecord,
  SurveyQueryParams,
  SurveyRecord,
} from './types';
import { normalizeSurveyCategory } from './types';

export const API_BASE = import.meta.env.VITE_API_BASE_URL ?? '';

type RawSurveyRecord = Omit<SurveyRecord, 'category'> & { category: string };

// ─── Auth ────────────────────────────────────────────────────────────────────
const TOKEN_KEY = 'ifem_token';

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token);
}

export function clearToken(): void {
  localStorage.removeItem(TOKEN_KEY);
}

function authHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function handleResponse<T>(res: Response): Promise<T> {
  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    try {
      const body = await res.json();
      msg = body.error ?? body.message ?? msg;
    } catch {
      // ignore
    }
    throw new Error(msg);
  }
  return res.json() as Promise<T>;
}

function normalizeSurveyRecord(record: RawSurveyRecord): SurveyRecord {
  return {
    ...record,
    category: normalizeSurveyCategory(record.category) ?? record.category,
  };
}

// ─── Login ───────────────────────────────────────────────────────────────────
export async function login(account: string, password: string): Promise<AuthBody> {
  const res = await fetch(`${API_BASE}/api/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ account, password }),
  });
  return handleResponse<AuthBody>(res);
}

// ─── Surveys ─────────────────────────────────────────────────────────────────
export async function listSurveys(params?: SurveyQueryParams): Promise<SurveyRecord[]> {
  const qs = new URLSearchParams();
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      if (v !== undefined && v !== '') qs.set(k, String(v));
    }
  }
  const res = await fetch(`${API_BASE}/api/surveys?${qs}`, {
    headers: { ...authHeaders() },
  });
  const records = await handleResponse<RawSurveyRecord[]>(res);
  return records.map(normalizeSurveyRecord);
}

export async function getSurvey(id: string): Promise<SurveyRecord> {
  const res = await fetch(`${API_BASE}/api/surveys/${id}`, {
    headers: { ...authHeaders() },
  });
  const record = await handleResponse<RawSurveyRecord>(res);
  return normalizeSurveyRecord(record);
}

export async function createSurvey(payload: CreateSurveyRequest): Promise<{ success: boolean; internal_id?: string }> {
  const res = await fetch(`${API_BASE}/api/surveys`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify(payload),
  });
  return handleResponse(res);
}

// ─── Photos ──────────────────────────────────────────────────────────────────
export async function listPhotos(surveyId: string): Promise<PhotoRecord[]> {
  const res = await fetch(`${API_BASE}/api/surveys/${surveyId}/photos`, {
    headers: { ...authHeaders() },
  });
  return handleResponse<PhotoRecord[]>(res);
}

export async function uploadPhotos(surveyId: string, files: File[]): Promise<string[]> {
  const form = new FormData();
  for (const f of files) form.append('photo', f, f.name);
  const res = await fetch(`${API_BASE}/api/surveys/${surveyId}/photos`, {
    method: 'POST',
    headers: { ...authHeaders() },
    body: form,
  });
  const data = await handleResponse<{ photo_ids: string[] }>(res);
  return data.photo_ids;
}

export async function deletePhoto(photoId: string): Promise<void> {
  const res = await fetch(`${API_BASE}/api/photos/${photoId}`, {
    method: 'DELETE',
    headers: { ...authHeaders() },
  });
  await handleResponse(res);
}

export function photoUrl(photoId: string): string {
  return `${API_BASE}/api/photos/${photoId}`;
}
