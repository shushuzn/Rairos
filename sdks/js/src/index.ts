/**
 * Rairos API JavaScript/TypeScript SDK
 *
 * Comprehensive SDK for interacting with the Rairos API.
 */

import {
  RairosError,
  AuthenticationError,
  RateLimitError,
  ValidationError,
  NotFoundError,
  ForbiddenError,
  ServerError,
  TimeoutError,
  ConnectionError,
  raiseFromResponse,
} from "./errors";

const BASE_URL = process.env.RAIROS_API_URL || "https://api.rairos.ai/api/v1";

interface RequestOptions {
  body?: Record<string, unknown>;
  params?: Record<string, string | number | undefined>;
  timeout?: number;
}

interface RetryConfig {
  maxRetries: number;
  retryDelay: number;
  statusCodesToRetry: number[];
}

const DEFAULT_RETRY_CONFIG: RetryConfig = {
  maxRetries: 3,
  retryDelay: 1000,
  statusCodesToRetry: [429, 500, 502, 503, 504],
};

export class RairosClient {
  private apiKey: string;
  private retryConfig: RetryConfig;

  constructor(apiKey?: string, retryConfig?: Partial<RetryConfig>) {
    this.apiKey = apiKey || process.env.RAIROS_API_KEY || "";
    if (!this.apiKey) {
      throw new AuthenticationError("API key is required");
    }

    this.retryConfig = {
      ...DEFAULT_RETRY_CONFIG,
      ...retryConfig,
    };
  }

  private async request<T>(
    method: string,
    endpoint: string,
    options: RequestOptions = {}
  ): Promise<T> {
    const { body, params, timeout = 30000 } = options;
    let lastError: Error | undefined;

    for (let attempt = 0; attempt <= this.retryConfig.maxRetries; attempt++) {
      try {
        return await this.doRequest<T>(method, endpoint, {
          body,
          params,
          timeout,
        });
      } catch (error) {
        lastError = error as Error;

        if (error instanceof RateLimitError && attempt < this.retryConfig.maxRetries) {
          const delay = this.retryConfig.retryDelay * Math.pow(2, attempt);
          const resetAt = (error as RateLimitError).resetAt;
          if (resetAt) {
            const waitUntil = resetAt.getTime() - Date.now();
            if (waitUntil > 0 && waitUntil < 60000) {
              await this.sleep(waitUntil);
              continue;
            }
          }
          await this.sleep(delay);
          continue;
        }

        if (
          error instanceof ServerError &&
          attempt < this.retryConfig.maxRetries &&
          this.retryConfig.statusCodesToRetry.includes((error as ServerError).statusCode || 0)
        ) {
          const delay = this.retryConfig.retryDelay * Math.pow(2, attempt);
          await this.sleep(delay);
          continue;
        }

        throw error;
      }
    }

    throw lastError || new RairosError("Max retries exceeded");
  }

  private async doRequest<T>(
    method: string,
    endpoint: string,
    options: RequestOptions
  ): Promise<T> {
    const { body, params, timeout } = options;

    const url = new URL(`${BASE_URL}${endpoint}`);
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
          url.searchParams.append(key, String(value));
        }
      });
    }

    const headers: Record<string, string> = {
      Authorization: `Bearer ${this.apiKey}`,
      "Content-Type": "application/json",
    };

    const fetchOptions: RequestInit = {
      method,
      headers,
      signal: timeout ? AbortSignal.timeout(timeout) : undefined,
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
        let errorData: Record<string, unknown> = {};
        try {
          errorData = await response.json();
        } catch {
          // ignore parse error
        }

        const error = errorData?.error as Record<string, unknown> || {};
        const limit = error?.limit as number | undefined;
        const resetAtStr = error?.reset_at as string | undefined;
        const resetAt = resetAtStr ? new Date(resetAtStr) : undefined;

        throw new RateLimitError(
          "Rate limit exceeded. Please retry after the reset time.",
          limit,
          resetAt,
          { limit, resetAt }
        );
      }

      if (!response.ok) {
        let errorData: Record<string, unknown> = {};
        try {
          errorData = await response.json();
        } catch {
          errorData = { error: { message: response.statusText || "Unknown error" } };
        }

        raiseFromResponse(response.status, errorData as any);
      }

      const text = await response.text();
      return text ? JSON.parse(text) : ({} as T);
    } catch (error) {
      if (error instanceof AuthenticationError ||
          error instanceof RateLimitError ||
          error instanceof RairosError) {
        throw error;
      }

      if (error instanceof Error) {
        if (error.name === "TimeoutError" || error.name === "AbortError") {
          throw new TimeoutError(`Request timed out: ${endpoint}`);
        }

        if (error.message.includes("fetch") || error.message.includes("network")) {
          throw new ConnectionError(`Connection failed: ${error.message}`);
        }

        throw new RairosError(`Request failed: ${error.message}`, "UNKNOWN_ERROR");
      }

      throw new RairosError("Unknown error occurred");
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
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

  async rotateKey(
    keyId: string,
    gracePeriodHours: number = 24
  ): Promise<{
    new_key: string;
    new_key_id: string;
    old_key_id: string;
    grace_period_ends: string;
    message: string;
  }> {
    return this.request("POST", "/keys/rotate", {
      body: { key_id: keyId, grace_period_hours: gracePeriodHours },
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

  async getUsageDashboard(): Promise<{
    total_requests: number;
    requests_today: number;
    requests_this_week: number;
    requests_this_month: number;
    limit: number;
    remaining: number;
    usage_percent: number;
    tier: string;
    reset_at: string;
    endpoint_breakdown: Array<{
      endpoint: string;
      count: number;
      avg_latency_ms: number;
      last_called: string;
    }>;
    daily_trend: Array<{
      date: string;
      count: number;
    }>;
  }> {
    return this.request("GET", "/usage/dashboard");
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
    const params: Record<string, string | number | undefined> = { page, per_page };
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
  async detectGap(
    query: string,
    extra: Record<string, unknown> = {}
  ): Promise<unknown> {
    return this.request("POST", "/gap/detect", {
      body: { query, ...extra },
    });
  }

  // Research (Team+)
  async runResearch(
    query: string,
    extra: Record<string, unknown> = {}
  ): Promise<unknown> {
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

export {
  RairosError,
  AuthenticationError,
  RateLimitError,
  ValidationError,
  NotFoundError,
  ForbiddenError,
  ServerError,
  TimeoutError,
  ConnectionError,
};

export default RairosClient;
