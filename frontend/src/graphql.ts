// Copyright 2026 Belgian Secure Communications (BSC)
// Copyright 2025, 2026 Element Creations Ltd.
// Copyright 2024, 2025 New Vector Ltd.
// Copyright 2023, 2024 The Matrix.org Foundation C.I.C.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.
//
// Modified by Belgian Secure Communications for Beam application on 2026-04-30

import type { ExecutionResult } from "graphql";
import type { TypedDocumentString } from "./gql/graphql";

let graphqlEndpoint = "/graphql";

/**
 * Sets the GraphQL endpoint to use for requests.
 * This is called during initialization, once config has been loaded.
 */
export const setGraphqlEndpoint = (endpoint: string): void => {
  graphqlEndpoint = endpoint;
};

// PG_CHANGED
let csrfToken: string | undefined;

/**
 * Sets the CSRF token sent along with GraphQL requests.
 * This is called during initialization, once config has been loaded.
 */
export const setCsrfToken = (token: string | undefined): void => {
  csrfToken = token;
};

type RequestOptions<TData, TVariables> = {
  query: TypedDocumentString<TData, TVariables>;
  signal?: AbortSignal;
  // biome-ignore lint/suspicious/noExplicitAny: this is for inference
} & (TVariables extends Record<any, never>
  ? { variables?: TVariables }
  : { variables: TVariables });

export const graphqlRequest = async <TData, TVariables>({
  query,
  variables,
  signal,
}: RequestOptions<TData, TVariables>): Promise<TData> => {
  const endpoint =
    import.meta.env.TEST && typeof window === "undefined"
      ? new URL(graphqlEndpoint, "http://localhost/").toString()
      : new URL(graphqlEndpoint, window.location.toString()).toString();

  // PG_CHANGED
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (csrfToken) {
    headers["X-CSRF-Token"] = csrfToken;
  }

  let response: Response;
  try {
    response = await fetch(endpoint, {
      method: "POST",
      headers,
      body: JSON.stringify({
        query,
        variables,
      }),
      signal,
    });
  } catch (cause) {
    throw new Error(`GraphQL request to ${endpoint} request failed`, {
      cause,
    });
  }

  if (!response.ok) {
    throw new Error(
      `GraphQL request to ${endpoint} failed: ${response.status}`,
    );
  }

  const json: ExecutionResult<TData> = await response.json();
  if (json.errors) {
    throw new Error(JSON.stringify(json.errors));
  }

  if (!json.data) {
    throw new Error(`GraphQL request to ${endpoint} returned no data`);
  }

  return json.data;
};
