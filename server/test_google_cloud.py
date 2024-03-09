import vertexai
from vertexai.preview.generative_models import GenerativeModel, Part
import vertexai.preview.generative_models as generative_models

def generate():
  vertexai.init(project="sevendrl-gemini", location="us-east4")
  model = GenerativeModel("gemini-1.0-pro-001")
  responses = model.generate_content(
    """You are the game master for a difficult permadeath roguelike. For each input theme and level, output JSON monster definitions. Valid types and attack types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. Valid colors are: lightgray yellow gold orange pink red maroon green lime skyblue blue purple violet beige brown white magenta. Output fields include name, the name of the monster; level, a number between 1 and 3 indicating how powerful the monster is; char, the single character to represent it as; color, one of the valid colors above; type1, the pokemon type of the monster; type2, an optional second type; attack_type, the pokemon the creature attacks as; and description, a two sentence description of the monster. Output each monster JSON on its own line.""",
    generation_config={
        "max_output_tokens": 2048,
        "temperature": 0.9,
        "top_p": 1,
        "stop_sequences": ["--"],
    },
    safety_settings={
          generative_models.HarmCategory.HARM_CATEGORY_HATE_SPEECH: generative_models.HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
          generative_models.HarmCategory.HARM_CATEGORY_DANGEROUS_CONTENT: generative_models.HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
          generative_models.HarmCategory.HARM_CATEGORY_SEXUALLY_EXPLICIT: generative_models.HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
          generative_models.HarmCategory.HARM_CATEGORY_HARASSMENT: generative_models.HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
    },
    stream=False,
  )

  try:
      text = responses.candidates[0].content.parts[0].text
      text = text.strip("--")
      print(text)
      return text
  except KeyError:
      print(response.json())
      raise


generate()
