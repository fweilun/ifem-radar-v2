import urllib.request
import urllib.error
import json
import uuid
from dataclasses import dataclass, asdict, field
from typing import Optional, List, Dict, Any
from enum import Enum

# Define the Enum for SurveyCategory
class SurveyCategory(str, Enum):
    ConnectingPipe = "ConnectingPipe"
    CrossingPipe = "CrossingPipe"
    BoxDamage = "BoxDamage"
    AttachmentLoss = "AttachmentLoss"
    Siltation = "Siltation"
    SectionChange = "SectionChange"
    CannotPass = "CannotPass"
    Unknown = "Unknown"

# Define ChangeOfArea Struct
@dataclass
class ChangeOfArea:
    width: float
    height: float
    change_width: float
    change_height: float

# Define SurveyDetails Struct
@dataclass
class SurveyDetails:
    diameter: Optional[int] = None
    length: Optional[float] = None
    width: Optional[float] = None
    protrusion: Optional[int] = None
    siltation_depth: Optional[int] = None
    crossing_pipe_count: Optional[int] = None
    change_of_area: Optional[ChangeOfArea] = None
    issues: Optional[List[str]] = None

# Define CreateSurveyRequest Struct
@dataclass
class SurveyRecordSender:
    """
    Python object to represent CreateSurveyRequest and transmit it to the server.
    """
    id: str
    start_point: str
    end_point: str
    orientation: str
    distance: float
    top_distance: str
    category: SurveyCategory
    details: SurveyDetails
    awaiting_photo_count: int
    remarks: Optional[str] = None
    
    # Helper to generate a new record with a random ID
    @classmethod
    def create_new(cls, 
                   start_point: str, 
                   end_point: str, 
                   orientation: str,
                   distance: float,
                   top_distance: str,
                   category: SurveyCategory,
                   details: SurveyDetails,
                   awaiting_photo_count: int = 0,
                   remarks: Optional[str] = None) -> 'SurveyRecordSender':
        return cls(
            id=str(uuid.uuid4()),
            start_point=start_point,
            end_point=end_point,
            orientation=orientation,
            distance=distance,
            top_distance=top_distance,
            category=category,
            details=details,
            awaiting_photo_count=awaiting_photo_count,
            remarks=remarks
        )

    def to_payload(self) -> Dict[str, Any]:
        """
        Converts the object to a dictionary suitable for JSON serialization.
        Handles Enum conversion and nested dataclasses.
        """
        data = asdict(self)
        
        # Manually ensure Enum is converted to string value
        data['category'] = self.category.value
        
        return data

    def send(self, base_url: str = "http://localhost:8080") -> None:
        """
        Transmits the information to the API endpoint using standard urllib.
        """
        endpoint = f"{base_url}/api/surveys"
        payload = self.to_payload()
        json_data = json.dumps(payload).encode('utf-8')
        
        print(f"Sending payload to {endpoint}...")
        
        req = urllib.request.Request(
            endpoint, 
            data=json_data, 
            headers={'Content-Type': 'application/json'}
        )
        
        try:
            with urllib.request.urlopen(req, timeout=10) as response:
                status_code = response.getcode()
                response_body = response.read().decode('utf-8')
                print(f"Status Code: {status_code}")
                try:
                    parsed_response = json.loads(response_body)
                    print(f"Response: {json.dumps(parsed_response, indent=2, ensure_ascii=False)}")
                except json.JSONDecodeError:
                    print(f"Response: {response_body}")
        except urllib.error.URLError as e:
            if hasattr(e, 'read'):
                 print(f"Error Response: {e.read().decode('utf-8')}")
            print(f"Error sending request: {e}")

    def upload_photo(self, file_path: str, base_url: str = "http://localhost:8080") -> None:
        """
        Uploads a photo for this survey record using multipart/form-data.
        """
        endpoint = f"{base_url}/api/surveys/{self.id}/photos"
        boundary = '----WebKitFormBoundary' + uuid.uuid4().hex
        
        try:
            with open(file_path, 'rb') as f:
                file_content = f.read()
            filename = file_path.split("/")[-1]
        except FileNotFoundError:
            print(f"File not found: {file_path}")
            return

        # Simple content type guessing
        content_type = 'application/octet-stream'
        if filename.lower().endswith('.png'):
            content_type = 'image/png'
        elif filename.lower().endswith(('.jpg', '.jpeg')):
            content_type = 'image/jpeg'
        elif filename.lower().endswith('.txt'):
            content_type = 'text/plain'

        # Construct multipart body manually
        parts = [
            f'--{boundary}'.encode('utf-8'),
            f'Content-Disposition: form-data; name="file"; filename="{filename}"'.encode('utf-8'),
            f'Content-Type: {content_type}'.encode('utf-8'),
            b'',
            file_content,
            f'--{boundary}--'.encode('utf-8'),
            b''
        ]
        
        # Join with CRLF
        body = b'\r\n'.join(parts)
        
        print(f"Uploading photo {filename} to {endpoint}...")
        
        req = urllib.request.Request(
            endpoint, 
            data=body, 
            headers={
                'Content-Type': f'multipart/form-data; boundary={boundary}',
                'Content-Length': str(len(body))
            }
        )
        
        try:
            with urllib.request.urlopen(req, timeout=30) as response:
                status_code = response.getcode()
                response_body = response.read().decode('utf-8')
                print(f"Upload Status Code: {status_code}")
                try:
                    parsed_response = json.loads(response_body)
                    print(f"Upload Response: {json.dumps(parsed_response, indent=2, ensure_ascii=False)}")
                except json.JSONDecodeError:
                    print(f"Upload Response: {response_body}")
        except urllib.error.URLError as e:
            if hasattr(e, 'read'):
                 print(f"Error Response: {e.read().decode('utf-8')}")
            print(f"Error uploading photo: {e}")

# Example Usage
if __name__ == "__main__":
    import os
    
    # Create details
    details = SurveyDetails(
        diameter=500,
        length=10.5,
        issues=["Crack", "Leakage"]
    )
    
    # Create the sender object
    sender = SurveyRecordSender.create_new(
        start_point="Manhole A",
        end_point="Manhole B",
        orientation="Downstream",
        distance=12.5,
        top_distance=">0",
        category=SurveyCategory.ConnectingPipe,
        details=details,
        remarks="Test survey record from Python (urllib)",
        awaiting_photo_count=2
    )

    # Transmit record
    sender.send()

    # Create a dummy photo file for testing
    dummy_photo_path = "test_photo.jpg"
    # Create a minimal valid JPG file or just bytes
    with open(dummy_photo_path, "wb") as f:
         # Minimal JPG header/footer
        f.write(b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00\x01\x01\x01\x00H\x00H\x00\x00\xFF\xDB\x00C\x00\x08\x06\x06\x07\x06\x05\x08\x07\x07\x07\t\t\x08\n\x0c\x14\r\x0c\x0b\x0b\x0c\x19\x12\x13\x0f\x14\x1d\x1a\x1f\x1e\x1d\x1a\x1c\x1c $.' \",#\x1c\x1c(7),01444\x1f'9=82<.342\xFF\xC0\x00\x11\x08\x00\x20\x00\x20\x03\x01\x22\x00\x02\x11\x01\x03\x11\x01\xFF\xC4\x00\x1F\x00\x00\x01\x05\x01\x01\x01\x01\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b\xFF\xDA\x00\x0C\x03\x01\x00\x02\x11\x03\x11\x00?\x00\xbf\x00\xFF\xD9")

    # Upload the photo
    sender.upload_photo(dummy_photo_path)
    sender.upload_photo(dummy_photo_path)
    
    # Clean up
    if os.path.exists(dummy_photo_path):
        os.remove(dummy_photo_path)
