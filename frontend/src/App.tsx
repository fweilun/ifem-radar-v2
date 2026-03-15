import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { getToken } from './api';
import CreateSurveyPage from './pages/CreateSurveyPage';
import LoginPage from './pages/LoginPage';
import SurveyDetailPage from './pages/SurveyDetailPage';
import SurveyListPage from './pages/SurveyListPage';

function RequireAuth({ children }: { children: React.ReactNode }) {
  return getToken() ? <>{children}</> : <Navigate to="/login" replace />;
}

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route
          path="/surveys"
          element={
            <RequireAuth>
              <SurveyListPage />
            </RequireAuth>
          }
        />
        <Route
          path="/surveys/new"
          element={
            <RequireAuth>
              <CreateSurveyPage />
            </RequireAuth>
          }
        />
        <Route
          path="/surveys/:id"
          element={
            <RequireAuth>
              <SurveyDetailPage />
            </RequireAuth>
          }
        />
        <Route path="*" element={<Navigate to={getToken() ? "/surveys" : "/login"} replace />} />
      </Routes>
    </BrowserRouter>
  );
}
