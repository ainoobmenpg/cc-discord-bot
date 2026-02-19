const express = require('express');
const { query } = require('@anthropic-ai/claude-agent-sdk');

const app = express();
app.use(express.json());

app.post('/query', async (req, res) => {
  const { prompt } = req.body;
  const messages = [];

  try {
    for await (const message of query({ prompt })) {
      messages.push(message);
    }
    res.json({ success: true, messages });
  } catch (error) {
    console.error('Error:', error);
    res.status(500).json({ success: false, error: error.message });
  }
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`Server running on http://localhost:${PORT}`);
});
