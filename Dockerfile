# slim Python image because b2b saas 
FROM python:3.11-slim

WORKDIR /app

# pip install dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# perform a business logic migration 
COPY app.py .

# expose port 8080 for cloud run
EXPOSE 8080

# run the business logic
CMD ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8080"]
