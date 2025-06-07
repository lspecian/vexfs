const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: false });
  const context = await browser.newContext();
  const page = await context.newPage();

  // Listen for console errors
  page.on('console', msg => {
    if (msg.type() === 'error') {
      console.log('Console Error:', msg.text());
    }
  });

  // Listen for page errors
  page.on('pageerror', error => {
    console.log('Page Error:', error.message);
    console.log('Stack:', error.stack);
  });

  try {
    console.log('Navigating to base UI...');
    await page.goto('http://localhost:7680/ui/');
    
    // Wait for the page to load
    await page.waitForTimeout(3000);
    
    console.log('Looking for Collections navigation...');
    
    // Try different selectors for Collections navigation
    const collectionsSelectors = [
      'a[href*="collections"]',
      'button:has-text("Collections")',
      'a:has-text("Collections")',
      '[data-testid="collections-nav"]',
      'nav a:has-text("Collections")',
      '.MuiTab-root:has-text("Collections")',
      '[role="tab"]:has-text("Collections")'
    ];
    
    let navigated = false;
    for (const selector of collectionsSelectors) {
      try {
        const element = await page.locator(selector).first();
        if (await element.isVisible()) {
          console.log(`Found Collections navigation with selector: ${selector}`);
          await element.click();
          navigated = true;
          await page.waitForTimeout(2000);
          break;
        }
      } catch (e) {
        // Continue to next selector
      }
    }
    
    if (!navigated) {
      console.log('Could not find Collections navigation, trying manual URL update...');
      // Try using page.evaluate to change the URL via client-side routing
      await page.evaluate(() => {
        if (window.history && window.history.pushState) {
          window.history.pushState({}, '', '/ui/collections');
          // Trigger a popstate event to notify React Router
          window.dispatchEvent(new PopStateEvent('popstate'));
        }
      });
      await page.waitForTimeout(2000);
    }
    
    console.log('Page loaded, checking for errors...');
    
    // Try to click the create collection button
    console.log('Looking for create collection button...');
    const createButton = await page.locator('button:has-text("Create Collection")').first();
    
    if (await createButton.isVisible()) {
      console.log('Found create button, clicking...');
      await createButton.click();
      
      // Wait for dialog to appear
      await page.waitForTimeout(1000);
      
      // Try to fill the form
      console.log('Trying to fill form...');
      
      // Check what form fields are available
      const nameInput = page.locator('input[name="name"]');
      const vectorSizeInput = page.locator('input[name="vectorSize"]');
      
      console.log('Checking form fields...');
      const nameVisible = await nameInput.isVisible();
      const vectorSizeVisible = await vectorSizeInput.isVisible();
      console.log(`Name input visible: ${nameVisible}, VectorSize input visible: ${vectorSizeVisible}`);
      
      if (nameVisible) {
        await nameInput.fill('Test_Collection');
        console.log('Filled name field with valid name (no spaces)');
      }
      
      if (vectorSizeVisible) {
        await vectorSizeInput.fill('384');
        console.log('Filled vectorSize field');
      }
      
      // Wait a moment for validation
      await page.waitForTimeout(1000);
      
      // Check submit button state
      const submitButton = await page.locator('button:has-text("Create Collection")').last();
      const isDisabled = await submitButton.getAttribute('disabled');
      const buttonClasses = await submitButton.getAttribute('class');
      console.log(`Submit button disabled: ${isDisabled !== null}, classes: ${buttonClasses}`);
      
      // Try to get form validation state
      const formErrors = await page.locator('.MuiFormHelperText-root.Mui-error').allTextContents();
      if (formErrors.length > 0) {
        console.log('Form validation errors:', formErrors);
      }
      
      // Try to submit if enabled
      if (isDisabled === null) {
        console.log('Submit button is enabled, clicking...');
        
        // Listen for network requests to see what's happening
        const requests = [];
        page.on('request', request => {
          if (request.url().includes('/api/')) {
            requests.push({
              method: request.method(),
              url: request.url(),
              postData: request.postData()
            });
          }
        });
        
        const responses = [];
        page.on('response', response => {
          if (response.url().includes('/api/')) {
            responses.push({
              status: response.status(),
              url: response.url(),
              statusText: response.statusText()
            });
          }
        });
        
        await submitButton.click();
        
        // Wait for submission to complete
        await page.waitForTimeout(3000);
        
        // Log API calls
        console.log('API Requests made:', JSON.stringify(requests, null, 2));
        console.log('API Responses received:', JSON.stringify(responses, null, 2));
        
        // Check for success/error messages
        const successMessage = await page.locator('.MuiAlert-message, .notistack-SnackbarContainer').first().textContent().catch(() => null);
        if (successMessage) {
          console.log('Success/Error message:', successMessage);
        }
        
        // Close the dialog if it's still open
        const closeButton = page.locator('button[aria-label="close"], button:has-text("Cancel")');
        if (await closeButton.isVisible()) {
          console.log('Closing create dialog...');
          await closeButton.click();
          await page.waitForTimeout(1000);
        }
        
        // Now check if the collection actually exists by calling the API directly
        console.log('Checking if collection was actually created...');
        const apiResponse = await page.evaluate(async () => {
          try {
            const response = await fetch('/api/v1/collections');
            const collections = await response.json();
            return { success: true, collections, status: response.status };
          } catch (error) {
            return { success: false, error: error.message };
          }
        });
        
        console.log('Direct API check result:', JSON.stringify(apiResponse, null, 2));
        
        if (apiResponse.success) {
          const testCollection = apiResponse.collections.find(c => c.name === 'Test_Collection');
          if (testCollection) {
            console.log('✅ Collection was actually created:', testCollection);
          } else {
            console.log('❌ Collection NOT found in API response');
            console.log('Available collections:', apiResponse.collections.map(c => c.name));
          }
        }
      } else {
        console.log('Submit button is disabled, cannot submit');
        
        // Try to trigger form validation by clicking anyway
        console.log('Attempting to click disabled button to see validation...');
        try {
          await submitButton.click({ timeout: 5000 });
        } catch (e) {
          console.log('Expected timeout on disabled button click');
        }
        
        // Close the dialog
        const closeButton = page.locator('button[aria-label="close"], button:has-text("Cancel")');
        if (await closeButton.isVisible()) {
          console.log('Closing create dialog...');
          await closeButton.click();
          await page.waitForTimeout(1000);
        }
      }
    } else {
      console.log('Create button not found');
    }
    
    // Try to test delete functionality
    console.log('Testing delete functionality...');
    const deleteButtons = await page.locator('button[aria-label*="Delete"], button:has-text("Delete")').all();
    if (deleteButtons.length > 0) {
      console.log(`Found ${deleteButtons.length} delete buttons, clicking first one...`);
      await deleteButtons[0].click();
      await page.waitForTimeout(1000);
    } else {
      console.log('No delete buttons found');
    }
    
  } catch (error) {
    console.log('Test Error:', error.message);
  }

  await page.waitForTimeout(5000);
  await browser.close();
})();