import os

import requests

API_URL = "https://api.mistral.ai"
API_KEY = os.getenv("AISTUDIO_API_KEY")


def ask_mistral(messages):
    model = "mistral-small"

    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": f"Bearer {API_KEY}",
    }

    payload = {"model": "mistral-tiny", "messages": messages, "max_tokens": 300}

    response = requests.post(
        f"{API_URL}/v1/chat/completions", headers=headers, json=payload
    )

    response.raise_for_status()

    return response.json()["choices"][0]["message"]["content"]


def ask_google(prompt_parts: list[str]) -> str:
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-1.0-pro:generateContent?key={API_KEY}"
    headers = {"Content-Type": "application/json"}
    payload = {
        "contents": [
            {
                "parts": [{"text": part} for part in prompt_parts],
            }
        ],
        "generationConfig": {
            "temperature": 0.9,
            "topK": 1,
            "topP": 1,
            "maxOutputTokens": 2048,
            "stopSequences": [],
        },
        "safetySettings": [
            {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_ONLY_HIGH"},
            {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_ONLY_HIGH"},
            {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "threshold": "BLOCK_ONLY_HIGH",
            },
            {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "threshold": "BLOCK_ONLY_HIGH",
            },
        ],
    }
    response = requests.post(url, headers=headers, json=payload)
    response.raise_for_status()
    return response.json()["candidates"][0]["content"]["parts"][0]["text"]


def ask_google_structured(input_fields: list[str], output_fields: list[str], instructions: str, examples: list[dict], input: dict) -> dict:
    prompt_parts = [instructions]
    for example in examples:
        for field in input_fields:
            prompt_parts.append(f"{field} {example[field]}")
        for field in output_fields:
            prompt_parts.append(f"{field} {example[field]}")
    for field in input_fields:
        prompt_parts.append(f"{field} {input[field]}")
    response_text = ask_google(prompt_parts)
    response_text = response_text.split(f"{output_fields[0]} ")[1]
    output = {}
    for i, field in enumerate(output_fields):
        if i == len(output_fields) - 1:
            output[field] = response_text
        else:
            next_field = output_fields[i+1]
            field_output, response_text = response_text.split(f"{next_field}")
            output[field] = field_output
    return output
