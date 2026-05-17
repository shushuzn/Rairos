/**
 * Rairos API JavaScript/TypeScript SDK
 */

const BASE_URL = process.env.RAIROS_API_URL || "https://api.rairos.ai/api/v1";

export class RairosError extends Error {
  statusCode?: number;

  constructor(message: string, statusCode?: number) {
    super(message);
    this.name = "RairosError";
    this.statusCode = statusCode;
  }
}

export class AuthenticationError extends RairosError {
  constructor(message: string = "Authentication failed") {
    super(message, 401);
    this.name = "AuthenticationError";
  }
}

export class RateLimitError extends RairosError {
  constructor(message: string = "Rate limit exceeded") {
    super(message, 429);
    this.name = "RateLimitError";
  }
}

export class RairosClient {
  private apiKey: string;

  constructor(apiKey?: string) {
    this.apiKey = apiKey || process.env.RAIROS_API_KEY || "";
    if (!this.apiKey) {
      throw new AuthenticationError("API key is required");
    }
  }

  private async request<T>(
    method: string,
    endpoint: string,
    options: {
      body?: Record<string, unknown>;
      params?: Record<string, string | number>;
    } = {}
  ): Promise<T> {
    const { body, params } = options;

    const url = new URL(`${BASE_URL}${endpoint}`);
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        url.searchParams.append(key, String(value));
      });
    }

    const headers: Record<string, string> = {
      Authorization: `Bearer ${this.apiKey}`,
      "Content-Type": "application/json",
    };

    const fetchOptions: RequestInit = {
      method,
      headers,
    };

    if (body) {
      fetchOptions.body = JSON.stringify(body);
    }

    try {
      const response = await fetch(url.toString(), fetchOptions);

      if (response.status === 401) {
        throw new AuthenticationError("Invalid or expired API key");
      }

      if (response.status === 429) {
        throw new RateLimitError("Rate limit exceeded");
      }

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        const errorMessage =
          errorData?.error?.message || `HTTP ${response.status}`;
        throw new RairosError(errorMessage, response.status);
      }

      return response.json();
    } catch (error) {
      if (error instanceof AuthenticationError || error instanceof RateLimitError || error instanceof RairosError) {
        throw error;
      }
      throw new RairosError(`Request failed: ${error}`);
    }
  }

  // Authentication
  async register(
    email: string,
    password: string
  ): Promise<{ user_id: string; email: string; api_key: string; tier: string }> {
    return this.request("POST", "/auth/register", {
      body: { email, password },
    });
  }

  async login(
    email: string,
    password: string
  ): Promise<{ user_id: string; email: string; api_key: string; tier: string }> {
    return this.request("POST", "/auth/login", {
      body: { email, password },
    });
  }

  // API Keys
  async listKeys(): Promise<Array<{
    id: string;
    name?: string;
    tier: string;
    requests_used: number;
    requests_limit: number;
  }>> {
    return this.request("GET", "/keys");
  }

  async createKey(name?: string): Promise<{
    id: string;
    api_key: string;
    name?: string;
    tier: string;
  }> {
    return this.request("POST", "/keys", {
      body: name ? { name } : {},
    });
  }

  // Usage
  async getUsage(): Promise<{
    tier: string;
    requests_used: number;
    requests_limit: number;
    requests_remaining: number;
    reset_at: string;
  }> {
    return this.request("GET", "/usage");
  }

  // Papers
  async searchPapers(options: {
    query?: string;
    page?: number;
    per_page?: number;
  } = {}): Promise<{
    papers: Array<{
      id: string;
      title: string;
      abstract: string;
      authors: string;
      categories: string;
      published: string;
    }>;
    page: number;
    per_page: number;
  }> {
    const { query, page = 1, per_page = 20 } = options;
    const params: Record<string, string | number> = { page, per_page };
    if (query) params.q = query;

    return this.request("GET", "/papers/search", { params });
  }

  async getPaper(paperId: string): Promise<{
    id: string;
    title: string;
    abstract: string;
    authors: string;
    categories: string;
    published: string;
  }> {
    return this.request("GET", `/papers/${paperId}`);
  }

  // Gap Detection (Pro+)
  async detectGap(query: string, extra: Record<string, unknown> = {}): Promise<unknown> {
    return this.request("POST", "/gap/detect", {
      body: { query, ...extra },
    });
  }

  // Research (Team+)
  async runResearch(query: string, extra: Record<string, unknown> = {}): Promise<unknown> {
    return this.request("POST", "/research/run", {
      body: { query, ...extra },
    });
  }

  // Subscriptions
  async getTiers(): Promise<{
    tiers: Array<{
      name: string;
      price_id: string;
      price_monthly: number;
      requests_limit: number;
    }>;
  }> {
    return this.request("GET", "/tiers");
  }

  async createCheckout(options: {
    priceId: string;
    successUrl?: string;
    cancelUrl?: string;
  }): Promise<{ checkout_url: string; session_id: string }> {
    const { priceId, successUrl, cancelUrl } = options;
    return this.request("POST", "/subscription/checkout", {
      body: {
        price_id: priceId,
        success_url: successUrl || "https://rairos.ai/success",
        cancel_url: cancelUrl || "https://rairos.ai/pricing",
      },
    });
  }

  async createPortal(returnUrl: string = "https://rairos.ai/dashboard"): Promise<{
    portal_url: string;
  }> {
    return this.request("POST", "/subscription/portal", {
      body: { return_url: returnUrl },
    });
  }

  async getSubscriptionStatus(): Promise<{
    tier: string;
    stripe_customer_id?: string;
    subscription_active: boolean;
  }> {
    return this.request("GET", "/subscription/status");
  }
}

export default RairosClient;
