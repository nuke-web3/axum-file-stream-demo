filename='tests/bufficorn-2521.png'
filetype='text/csv'
token='my oauth token'
url='http://127.0.0.1:3000/file/upload'

curl "$url" \
  -iv --raw \
  --form "data=@$filename;type=$filetype" \
  --form "name=$filename" \
  -H "Authorization: Bearer $token"
  
