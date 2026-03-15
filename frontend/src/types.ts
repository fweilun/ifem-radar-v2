// ─── Survey Category Enum ────────────────────────────────────────────────────
export type SurveyCategory =
  | 'connecting_pipe'   // 連接管
  | 'crossing_pipe'     // 橫越管
  | 'box_damage'        // 箱涵破損
  | 'attachment_loss'   // 附掛缺失
  | 'siltation'         // 淤積
  | 'section_change'    // 斷面變化
  | 'cannot_pass';      // 無法縱走

export const CATEGORY_LABELS: Record<SurveyCategory, string> = {
  connecting_pipe: '連接管',
  crossing_pipe: '橫越管',
  box_damage: '箱涵破損',
  attachment_loss: '附掛缺失',
  siltation: '淤積',
  section_change: '斷面變化',
  cannot_pass: '無法縱走',
};

export const CATEGORY_VALUES = Object.keys(CATEGORY_LABELS) as SurveyCategory[];

const CATEGORY_ALIASES: Record<string, SurveyCategory> = {
  connecting_pipe: 'connecting_pipe',
  crossing_pipe: 'crossing_pipe',
  box_damage: 'box_damage',
  attachment_loss: 'attachment_loss',
  siltation: 'siltation',
  section_change: 'section_change',
  cannot_pass: 'cannot_pass',
  ConnectingPipe: 'connecting_pipe',
  CrossingPipe: 'crossing_pipe',
  BoxDamage: 'box_damage',
  AttachmentLoss: 'attachment_loss',
  Siltation: 'siltation',
  SectionChange: 'section_change',
  CannotPass: 'cannot_pass',
};

export function normalizeSurveyCategory(category: string): SurveyCategory | null {
  return CATEGORY_ALIASES[category] ?? null;
}

export function getCategoryLabel(category: string): string {
  const normalized = normalizeSurveyCategory(category);
  return normalized ? CATEGORY_LABELS[normalized] : category;
}

// ─── Field Map ──────────────────────────────────────────────────────────────
// dict: named input fields (number/text); list: multi-select chips
export interface FieldDef {
  dict: string[];
  list: string[];
}

export const FIELD_MAP: Record<SurveyCategory, FieldDef> = {
  connecting_pipe: {
    dict: ['直徑', '長', '寬', '凸出', '淤積'],
    list: ['PVC', '混凝土', '脫管', '接合破損'],
  },
  crossing_pipe: {
    dict: ['直徑', '根'],
    list: ['PVC', '鐵', '接合破損', '單位'],
  },
  box_damage: {
    dict: ['寬度', '長度'],
    list: ['鋼筋裸露', '牆體', '伸縮縫', '裂縫', '濕潤', '滲水', '漏水'],
  },
  attachment_loss: {
    dict: [],
    list: ['正常', '垂落', '橫越', '雜亂'],
  },
  siltation: {
    dict: ['寬度', '長度'],
    list: ['鋼筋裸露', '牆體', '伸縮縫', '裂縫', '濕潤', '滲水', '漏水'],
  },
  section_change: {
    dict: ['跌降', '變化'],
    list: [],
  },
  cannot_pass: {
    dict: [],
    list: ['氣體', '管徑', '淤積', '積水', '其他'],
  },
};

export function getFieldDefForCategory(category: string): FieldDef | null {
  const normalized = normalizeSurveyCategory(category);
  return normalized ? FIELD_MAP[normalized] : null;
}

// Mapping from Chinese label to SurveyDetails field name
export const DICT_FIELD_MAP: Record<string, keyof SurveyDetails> = {
  直徑: 'diameter',
  長: 'length',
  長度: 'length',
  寬: 'width',
  寬度: 'width',
  凸出: 'protrusion',
  淤積: 'siltation_depth',
  根: 'crossing_pipe_count',
  跌降: 'drop_height',
  變化: 'section_change_value',
};

// ─── Data Models ─────────────────────────────────────────────────────────────
export interface ChangeOfArea {
  width: number;
  height: number;
  change_width: number;
  change_height: number;
}

export interface SurveyDetails {
  diameter?: number;
  length?: number;
  width?: number;
  protrusion?: number;
  siltation_depth?: number;
  crossing_pipe_count?: number;
  change_of_area?: ChangeOfArea;
  issues?: string[];
  // virtual fields used in form (mapped to change_of_area)
  drop_height?: number;
  section_change_value?: number;
}

export interface SurveyRecord {
  id: string;
  start_point: string;
  end_point: string;
  orientation: string;
  distance: number;
  top_distance: string;
  category: string;
  details: SurveyDetails;
  remarks?: string;
  created_at?: string;
}

export interface CreateSurveyRequest {
  id: string;
  start_point: string;
  end_point: string;
  orientation: string;
  distance: number;
  top_distance: string;
  category: SurveyCategory;
  details: SurveyDetails;
  remarks?: string;
}

export interface PhotoRecord {
  id: string;
  survey_id: string;
  filename: string;
  content_type: string;
  created_at?: string;
}

export interface ApiResponse {
  success: boolean;
  message: string;
  internal_id?: string;
}

export interface AuthBody {
  access_token: string;
  token_type: string;
}

export interface SurveyQueryParams {
  category?: string;
  start_point?: string;
  end_point?: string;
  created_from?: string;
  created_to?: string;
  limit?: number;
  offset?: number;
}
