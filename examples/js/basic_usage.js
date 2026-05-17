/**
 * Basic usage example for Rairos JavaScript SDK.
 *
 * Run with: node examples/js/basic_usage.js
 *
 * Set RAIROS_API_KEY environment variable:
 *   export RAIROS_API_KEY=your_key_here
 */

const { RairosClient, RairosError } = require('rairos');

async function main() {
  // Initialize client with API key from environment
  const apiKey = process.env.RAIROS_API_KEY;
  if (!apiKey) {
    console.error('Error: RAIROS_API_KEY environment variable not set');
    console.log('Set it with: export RAIROS_API_KEY=your_key_here');
    return;
  }

  const client = new RairosClient(apiKey);

  // Search for papers
  console.log('='.repeat(50));
  console.log('Searching for papers about machine learning...');
  console.log('='.repeat(50));

  try {
    const results = await client.searchPapers({ query: 'machine learning', page: 1, perPage: 5 });
    console.log(`Found ${results.total || 0} papers\n`);

    const papers = results.papers || [];
    papers.slice(0, 5).forEach((paper, i) => {
      console.log(`${i + 1}. ${paper.title || 'N/A'}`);
      const authors = paper.authors || [];
      if (authors.length > 0) {
        const authorList = authors.slice(0, 3).join(', ');
        console.log(`   Authors: ${authorList}${authors.length > 3 ? '...' : ''}`);
      }
      console.log(`   Score: ${paper.score || 'N/A'}`);
      console.log();
    });

  } catch (error) {
    if (error instanceof RairosError) {
      console.error(`Search failed: ${error.message}`);
    } else {
      console.error('Unexpected error:', error);
    }
  }

  // Check usage
  console.log('='.repeat(50));
  console.log('Checking API usage...');
  console.log('='.repeat(50));

  try {
    const usage = await client.getUsage();
    console.log(`Tier: ${usage.tier || 'N/A'}`);
    console.log(`Requests used: ${usage.requestsUsed || 0}`);
    console.log(`Requests limit: ${usage.requestsLimit || 'unlimited'}`);
    console.log(`Rate limit per minute: ${usage.rateLimitPerMinute || 'N/A'}`);

    // Calculate percentage if limit exists
    if (usage.requestsLimit && usage.requestsLimit !== 'unlimited') {
      const used = usage.requestsUsed || 0;
      const limit = usage.requestsLimit;
      const pct = ((used / limit) * 100).toFixed(1);
      console.log(`Usage: ${pct}%`);
    }

  } catch (error) {
    if (error instanceof RairosError) {
      console.error(`Usage check failed: ${error.message}`);
    } else {
      console.error('Unexpected error:', error);
    }
  }
}

main().catch(console.error);
