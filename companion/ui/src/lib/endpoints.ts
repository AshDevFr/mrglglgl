// Services that answer the OpenAI-compatible chat API at a known address, so picking one fills
// the address in. Anything else that speaks the same API (a LiteLLM proxy, vLLM, a provider not
// listed) works through Other with its own address.

export interface EndpointPreset {
  id: string;
  name: string;
  url: string;
}

export const ENDPOINT_PRESETS: EndpointPreset[] = [
  { id: "ollama", name: "Ollama", url: "http://localhost:11434/v1" },
  { id: "lmstudio", name: "LM Studio", url: "http://localhost:1234/v1" },
  { id: "llamacpp", name: "llama.cpp server", url: "http://localhost:8080/v1" },
  { id: "openai", name: "OpenAI", url: "https://api.openai.com/v1" },
  { id: "openrouter", name: "OpenRouter", url: "https://openrouter.ai/api/v1" },
  { id: "mistral", name: "Mistral", url: "https://api.mistral.ai/v1" },
  { id: "groq", name: "Groq", url: "https://api.groq.com/openai/v1" },
];

export const OTHER = "other";

/** The preset an address is, ignoring a trailing slash, or Other. */
export function presetFor(url: string): string {
  const plain = url.trim().replace(/\/+$/, "");
  return ENDPOINT_PRESETS.find((p) => p.url === plain)?.id ?? OTHER;
}
