import { HfAgent } from "@huggingface/agents";
import dotenv from "dotenv";
import type { Tool } from "@huggingface/agents/src/types";

dotenv.config();
type Data = string | Blob | ArrayBuffer;

const tools: Tool[] = [
  {
    name: "createRequest",
    description:
      "Create a new request with name and description, convert arguments to JSON.",
    examples: [
      {
        prompt:
          "Create a request with name Gucci Bag and description Nice Bag .",
        code: `const output = createRequest('{"name":"Gucci Bag","description":"Nice Bag"}')`,
        tools: ["createRequest"],
      },
    ],
    call: async (input: Promise<Data>): Promise<Data> => {
      const data = await input;

      const { name, description } = JSON.parse(data.toString());
      return JSON.stringify({
        name: "createRequest",
        args: {
          name,
          description,
        },
        type: "tool_call",
        id: 1,
      });
    },
  },
  {
    name: "getRequest",
    description: "Get or Fetch a request by ID.",
    examples: [
      {
        prompt: "Get details for request ID 123.",
        code: `const output = getRequest('{ id: "123" }')`,
        tools: ["getRequest"],
      },
    ],
    call: async (input: Promise<Data>): Promise<Data> => {
      const data = await input;

      const { id } = JSON.parse(data.toString());
      return JSON.stringify({
        name: "getRequest",
        args: {
          id,
        },
        type: "tool_call",
        id: 2,
      });
    },
  },
  {
    name: "createStore",
    description: "Create a new store.",
    examples: [
      {
        prompt:
          "Create a store with name Kingsmen and description we are best.",
        code: `const output = createStore('{"name":"Kingsmen","description":"we are best"}')`,
        tools: ["createStore"],
      },
    ],
    call: async (input: Promise<Data>): Promise<Data> => {
      const data = await input;

      const { name, description } = JSON.parse(data.toString());
      return JSON.stringify({
        name: "createStore",
        args: {
          name,
          description,
        },
        type: "tool_call",
        id: 3,
      });
    },
  },
  {
    name: "createUser",
    description: "Create a new user, account type must be seller or buyer.",
    examples: [
      {
        prompt:
          "Create a new buyer user with name john_doe and phone 1234567890, account type buyer.",
        code: `const output = createUser('{"username":"john_doe","account_type":"buyer","phone":"1234567890"}')`,
        tools: ["createUser"],
      },
    ],

    call: async (input: Promise<Data>): Promise<Data> => {
      const data = await input;

      const { username, account_type, phone } = JSON.parse(data.toString());
      return JSON.stringify({
        name: "createUser",
        args: {
          username,
          account_type,
          phone,
        },
        type: "tool_call",
        id: 4,
      });
    },
  },
  {
    name: "updateUser",
    description: "Update an existing user.",
    examples: [
      {
        prompt: "Update user information.",
        code: `const output = updateUser({})`,
        tools: ["updateUser"],
      },
    ],
    call: async (input: Promise<Data>): Promise<Data> => {
      const data = await input;

      const { _ } = JSON.parse(data.toString());
      return JSON.stringify({
        name: "updateUser",
        args: {},
        type: "tool_call",
        id: 5,
      });
    },
  },
  {
    name: "toggleLocation",
    description: "Enable or disable location tracking.",
    examples: [
      {
        prompt: "Enable location tracking.",
        code: `const output = toggleLocation('{ "isEnabled": true }')`,
        tools: ["toggleLocation"],
      },
    ],
    call: async (input: Promise<Data>): Promise<Data> => {
      const data = await input;

      const { isEnabled } = JSON.parse(data.toString());
      return JSON.stringify({
        name: "toggleLocation",
        args: {
          isEnabled,
        },
        type: "tool_call",
        id: 6,
      });
    },
  },
  {
    name: "fetchUserById",
    description: "Get/Fetch user details by ID.",
    examples: [
      {
        prompt: "Get/Fetch details for user ID 456.",
        code: `const output = fetchUserById('{ id: "456" }')`,
        tools: ["fetchUserById"],
      },
      {
        prompt: "Get user 382.",
        code: `const output = fetchUserById('{ id: "382" }')`,
        tools: ["fetchUserById"],
      },
    ],
    call: async (input: Promise<Data>): Promise<Data> => {
      const data = await input;

      const { id } = JSON.parse(data.toString());
      return JSON.stringify({
        name: "fetchUserById",
        args: {
          id,
        },
        type: "tool_call",
        id: 7,
      });
    },
  },
];

export async function runAIAgent(messages: string) {
  try {
    const agent = new HfAgent(
      process.env.HUGGINGFACE_API_KEY!,
      undefined,
      tools
    );
    const response = await agent.run(messages);
    const results = [];
    for (const res of response) {
      try {
        results.push(JSON.parse(res.data as string));
      } catch (error) {}
    }
    return { content: "", tool_calls: results };
  } catch (error) {
    console.error(error);
    return {};
  }
}
