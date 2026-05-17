/**
 * Rairos SDK Errors
 *
 * Comprehensive error types for the Rairos API.
 */

export interface ErrorDetails {
  limit?: number;
  resetAt?: Date;
  code?: string;
  [key: string]: unknown;
}

export class RairosError extends Error {
  public readonly code: string;
  public readonly statusCode?: number;
  public readonly details: ErrorDetails;

  constructor(
    message: string,
    code: string = "ERROR",
    statusCode?: number,
    details: ErrorDetails = {}
  ) {
    super(message);
    this.name = "RairosError";
    this.code = code;
    this.statusCode = statusCode;
    this.details = details;
  }

  toString(): string {
    return this.code !== "ERROR"
      ? `[${this.code}] ${this.message}`
      : this.message;
  }
}

export class AuthenticationError extends RairosError {
  constructor(
    message: string = "Authentication failed",
    details: ErrorDetails = {}
  ) {
    super(message, "AUTH", 401, details);
    this.name = "AuthenticationError";
  }
}

export class RateLimitError extends RairosError {
  public readonly limit?: number;
  public readonly resetAt?: Date;

  constructor(
    message: string = "Rate limit exceeded",
    limit?: number,
    resetAt?: Date,
    details: ErrorDetails = {}
  ) {
    super(message, "RATE_LIMITED", 429, { limit, resetAt, ...details });
    this.name = "RateLimitError";
    this.limit = limit;
    this.resetAt = resetAt;
  }

  toString(): string {
    const parts = [`[RATE_LIMITED] ${this.message}`];
    if (this.limit) parts.push(`Limit: ${this.limit}`);
    if (this.resetAt) parts.push(`Resets at: ${this.resetAt.toISOString()}`);
    return parts.join(" | ");
  }
}

export class ValidationError extends RairosError {
  constructor(
    message: string = "Validation failed",
    details: ErrorDetails = {}
  ) {
    super(message, "VALIDATION_ERROR", 400, details);
    this.name = "ValidationError";
  }
}

export class NotFoundError extends RairosError {
  constructor(
    message: string = "Resource not found",
    details: ErrorDetails = {}
  ) {
    super(message, "NOT_FOUND", 404, details);
    this.name = "NotFoundError";
  }
}

export class ForbiddenError extends RairosError {
  constructor(
    message: string = "Access forbidden",
    details: ErrorDetails = {}
  ) {
    super(message, "FORBIDDEN", 403, details);
    this.name = "ForbiddenError";
  }
}

export class PaymentError extends RairosError {
  constructor(
    message: string = "Payment error",
    details: ErrorDetails = {}
  ) {
    super(message, "PAYMENT_ERROR", 402, details);
    this.name = "PaymentError";
  }
}

export class ServerError extends RairosError {
  constructor(
    message: string = "Internal server error",
    details: ErrorDetails = {}
  ) {
    super(message, "SERVER_ERROR", 500, details);
    this.name = "ServerError";
  }
}

export class TimeoutError extends RairosError {
  constructor(
    message: string = "Request timed out",
    details: ErrorDetails = {}
  ) {
    super(message, "TIMEOUT", undefined, details);
    this.name = "TimeoutError";
  }
}

export class ConnectionError extends RairosError {
  constructor(
    message: string = "Connection failed",
    details: ErrorDetails = {}
  ) {
    super(message, "CONNECTION_ERROR", undefined, details);
    this.name = "ConnectionError";
  }
}

interface ErrorResponse {
  error?: {
    code?: string;
    message?: string;
    limit?: number;
    reset_at?: string;
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

export function parseErrorResponse(data: ErrorResponse): {
  code: string;
  details: ErrorDetails;
} {
  const error = data?.error || {};
  return {
    code: error?.code || "ERROR",
    details: {
      limit: error?.limit,
      resetAt: error?.reset_at ? new Date(error.reset_at) : undefined,
    },
  };
}

export function raiseFromResponse(statusCode: number, data: ErrorResponse): never {
  const { code, details } = parseErrorResponse(data);
  const message = data?.error?.message || "Unknown error";

  switch (statusCode) {
    case 400:
      throw new ValidationError(message, details);
    case 401:
      throw new AuthenticationError(message, details);
    case 403:
      throw new ForbiddenError(message, details);
    case 404:
      throw new NotFoundError(message, details);
    case 402:
      throw new PaymentError(message, details);
    case 429:
      throw new RateLimitError(
        message || "Rate limit exceeded",
        details.limit,
        details.resetAt,
        details
      );
    default:
      if (statusCode >= 500) {
        throw new ServerError(message, details);
      }
      throw new RairosError(message, code, statusCode, details);
  }
}
